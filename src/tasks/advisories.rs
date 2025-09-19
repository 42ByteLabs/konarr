//! # Task - Advisories

use super::{AlertCalculatorTask, TaskTrait};
use crate::{
    Config, KonarrError,
    bom::{BomParser, Parsers},
    models::{
        Alerts, Projects, ServerSettings, Setting, dependencies::snapshots::SnapshotState,
        security::SecurityState,
    },
    tools::{Grype, Tool, ToolConfig},
};
use async_trait::async_trait;
use geekorm::{ConnectionManager, prelude::*};
use log::{debug, info, warn};

/// Advisories Task to scan for security alerts
#[derive(Default)]
pub struct AdvisoriesTask {}

#[async_trait]
impl TaskTrait for AdvisoriesTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        self.scan(database).await?;

        AlertCalculatorTask::spawn(database).await?;

        Ok(())
    }
}

impl<'a> AdvisoriesTask {
    /// Create a new Advisories Task
    pub fn new(_config: &'a Config) -> Result<Self, KonarrError> {
        Ok(Self {})
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

        let mut projects = Projects::fetch_containers(&database.acquire().await).await?;
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

        let sbom_dir = std::env::temp_dir().join("konarr");
        if !sbom_dir.exists() {
            tokio::fs::create_dir_all(&sbom_dir).await?;
        }

        if let Some(snapshot) = project.snapshots.first() {
            if snapshot.state == SnapshotState::Failed {
                warn!("Snapshot is in failed state, skipping");
                return Ok(());
            }

            let mut snapshot = snapshot.clone();
            info!(
                "Scanning Snapshot :: {} - {}",
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
            let sbom_data = if let Ok(data) = snapshot.sbom(&database.acquire().await).await {
                data
            } else {
                warn!("No SBOM data for project: {}", project.name);
                snapshot.rescan(&database.acquire().await).await?;
                return Ok(());
            };

            let sbom = Parsers::parse(&sbom_data)?;
            log::info!("BillOfMaterials(comps='{}')", sbom.components.len(),);

            let sbom_file = uuid::Uuid::new_v4().to_string();
            let sbom_path =
                sbom_dir.join(format!("{}.{}", sbom_file, sbom.sbom_type.to_file_name()));

            // The SBOM needs to be written to disk to be scanned by Grype
            tokio::fs::write(&sbom_path, sbom_data).await?;

            log::debug!("Using Grype to scan SBOM: {}", sbom_path.display());

            let bom = tool_config.run(&sbom_path.display().to_string()).await?;

            let sbom = Parsers::parse(&bom.as_bytes())?;
            log::info!(
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

            // Remove re-scan alerts metadata
            snapshot
                .set_metadata(&database.acquire().await, "rescan", "false")
                .await?;

            snapshot.updated_at = Some(chrono::Utc::now());
            snapshot.state = SnapshotState::Completed;
            snapshot.update(&database.acquire().await).await?;

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
