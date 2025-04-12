//! # Task - Advisories

use std::path::PathBuf;

use crate::{
    Config, KonarrError,
    bom::{BomParser, Parsers},
    models::{Alerts, Projects, ServerSettings, Setting, security::SecurityState},
    tools::{Grype, Tool, ToolConfig},
};
use async_trait::async_trait;
use geekorm::{ConnectionManager, prelude::*};
use log::{debug, info, warn};

use super::TaskTrait;

/// Advisories Task to scan for security alerts
pub struct AdvisoriesTask {
    grype_path: PathBuf,
    sboms_path: PathBuf,
}

#[async_trait]
impl TaskTrait for AdvisoriesTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        self.scan(database).await?;
        Ok(())
    }
}

impl<'a> AdvisoriesTask {
    /// Create a new Advisories Task
    pub fn new(config: &'a Config) -> Result<Self, KonarrError> {
        Ok(Self {
            grype_path: config.grype_path()?,
            sboms_path: config.sboms_path()?,
        })
    }
}

impl Default for AdvisoriesTask {
    fn default() -> Self {
        Self {
            grype_path: PathBuf::from("/var/lib/konarr/grype"),
            sboms_path: PathBuf::from("/var/lib/konarr/sboms"),
        }
    }
}

impl AdvisoriesTask {
    /// Scan for security alerts
    pub async fn scan(&self, database: &geekorm::ConnectionManager) -> Result<(), KonarrError> {
        if ServerSettings::get_bool(&database.acquire().await, Setting::Security).await? {
            log::info!("Scanning projects for security alerts");

            self.scan_projects(database).await?;
            Ok(())
        } else {
            Err(KonarrError::UnknownError(
                "Advisories Polling is disabled".to_string(),
            ))
        }
    }

    /// Scan every project for security alerts
    pub async fn scan_projects(
        &self,
        database: &geekorm::ConnectionManager,
    ) -> Result<(), KonarrError> {
        info!("Scanning projects snapshots for security alerts");

        // Ensure Grype is installed and available
        let mut tool_grype = Grype::init().await;
        if !tool_grype.is_available() {
            warn!("Installing Grype, this may take a few minutes...");
            tool_grype.install().await?;
        }
        log::debug!("Grype Config: {:?}", tool_grype);

        let mut projects = Projects::all(&database.acquire().await).await?;
        info!("Projects Count: {}", projects.len());

        for project in projects.iter_mut() {
            self.scan_project(database, &tool_grype, project).await?;
        }

        Ok(())
    }

    /// Scan a project for security alerts
    pub async fn scan_project(
        &self,
        database: &geekorm::ConnectionManager,
        tool_config: &ToolConfig,
        project: &mut Projects,
    ) -> Result<(), KonarrError> {
        debug!("Project: {}", project.name);

        if let Some(snapshot) = project.snapshots.first() {
            let mut snapshot = snapshot.clone();
            info!(
                "Scanning Snapshot ::: {} - {}",
                snapshot.id,
                snapshot.components.len()
            );

            // Fetch the alerts for the snapshot (previously stored)
            let mut alerts =
                Alerts::fetch_by_snapshot_id(&database.acquire().await, snapshot.id).await?;

            if let Some(tool_alerts) = snapshot.find_metadata("security.tools.alerts") {
                if tool_alerts.as_bool() {
                    // If the `tool alerts` setting is disabled, we
                    if ServerSettings::get_bool(
                        &database.acquire().await,
                        Setting::SecurityToolsAlerts,
                    )
                    .await?
                    {
                        info!(
                            "Project('{}', snapshot = '{}', components = '{}', vulnerabilities = '{}')",
                            project.name,
                            snapshot.id,
                            snapshot.components.len(),
                            alerts.len()
                        );
                        info!("Security Alerts coming from tools, skipping");
                        return Ok(());
                    } else {
                        info!(
                            "Security Tools Alerts setting is disabled, scanning project for security alerts"
                        );
                    }
                }
            }

            let mut results = Vec::new();

            // SBOM Data
            let sbom_data = snapshot.sbom(&database.acquire().await).await?;
            let sbom_path = snapshot.sbom_path(&database.acquire().await).await?;

            let full_path = self.sboms_path.join(sbom_path.as_str());
            if !full_path.exists() {
                warn!("SBOM does not exist: {}", full_path.display());
                return Ok(());
            }
            log::debug!("Using Grype to scan SBOM: {}", full_path.display());

            // The SBOM needs to be written to disk to be scanned by Grype
            tokio::fs::write(&full_path, sbom_data).await?;

            let bom = tool_config.run(&full_path.display().to_string()).await?;
            let sbom = Parsers::parse(bom.as_bytes())?;
            log::debug!(
                "BillOfMaterials(comps='{}', vulns='{}')",
                sbom.components.len(),
                sbom.vulnerabilities.len()
            );

            for vuln in sbom.vulnerabilities.iter() {
                log::trace!("Vulnerability: {:?}", vuln);
                let alts =
                    Alerts::from_bom_vulnerability(&database.acquire().await, &snapshot, vuln)
                        .await?;
                results.extend(alts);
            }

            // Find all the alerts that are not in results
            for alert in alerts.iter_mut() {
                if !results.iter().any(|r| r.id == alert.id) {
                    debug!("Marking Alert as Resolved: {}", alert.id);
                    alert.state = SecurityState::Secure;
                    alert.update(&database.acquire().await).await?;
                }
            }

            // TODO: Cleanup

            // log::debug!("Removing SBOM: {}", full_path.display());
            // tokio::fs::remove_file(full_path).await?;

            info!(
                "Project('{}', snapshot = '{}', components = '{}', vulnerabilities = '{}')",
                project.name,
                snapshot.id,
                snapshot.components.len(),
                results.len()
            );
        } else {
            warn!("No snapshots for project: {}", project.name);
        }

        Ok(())
    }
}
