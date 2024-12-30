use std::collections::HashMap;

use geekorm::prelude::*;
use log::debug;

use super::GrypeDatabase;
use crate::{
    models::{
        dependencies::snapshots::AlertsSummary,
        security::{Advisories, AdvisorySource, Alerts, SecuritySeverity},
        Dependencies, Snapshot,
    },
    utils::grypedb::GrypeVulnerabilityMetadata,
};

impl GrypeDatabase {
    /// Fetch Grype Results for the Snapshot
    pub async fn matcher<'a, T>(
        connection: &'a T,
        grypedb: &GrypeDatabase,
        snapshot: &mut Snapshot,
    ) -> Result<Vec<Alerts>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut results = Vec::new();
        // TODO: Current alerts checking
        // let current_alerts = Alerts::fetch_by_snapshot_id(connection, snapshot.id).await?;

        // Fetch Dependencies if not present
        if snapshot.components.is_empty() {
            debug!("Fetching components as none are present");

            snapshot.components =
                Dependencies::fetch_by_snapshot_id(connection, snapshot.id).await?;
            for comp in snapshot.components.iter_mut() {
                comp.fetch(connection).await?;
            }
        }
        debug!("Dependencies Count: {}", snapshot.components.len());

        // Summary of the Security Alerts (cached)
        let mut summary: AlertsSummary = HashMap::new();

        for dependency in snapshot.components.iter_mut() {
            log::debug!(
                "Scanning Dependency: {}",
                dependency.component_id.data.purl()
            );
            let vulns = grypedb.find_vulnerability(
                &dependency.component_id.data,
                &dependency.component_version_id.data,
            )?;
            log::debug!(
                "Grype Results for {}@{} :: {}",
                dependency.component_id.data.purl(),
                dependency.component_version_id.data.version,
                vulns.len()
            );

            for vuln in &vulns {
                debug!("Vulnerability: {:?}", vuln);

                let vuln_metadata: Option<GrypeVulnerabilityMetadata> =
                    if vuln.id.starts_with("CVE-") {
                        // Look for the metadata in GrypeDB based on ID + namespace
                        GrypeVulnerabilityMetadata::query_first(
                            &grypedb.connection,
                            GrypeVulnerabilityMetadata::query_select()
                                .where_eq("id", &vuln.id)
                                .and()
                                // TODO: Support multiple namespaces
                                .where_eq("namespace", "nvd:cpe")
                                .build()?,
                        )
                        .await
                        .ok()
                    } else if vuln.id.starts_with("GHSA-") {
                        GrypeVulnerabilityMetadata::query_first(
                            &grypedb.connection,
                            GrypeVulnerabilityMetadata::query_select()
                                .where_eq("id", &vuln.id)
                                .and()
                                .where_like("namespace", format!("{}%", "github:"))
                                .build()?,
                        )
                        .await
                        .ok()
                    } else {
                        debug!("Skipping non-supported VULN IDs: {}", vuln.id);
                        continue;
                    };

                let mut severity = SecuritySeverity::Unknown;

                let mut advisory = if let Some(vuln_metadata) = vuln_metadata {
                    debug!("Vulnerability Metadata: {:?}", vuln_metadata);
                    severity = SecuritySeverity::from(vuln_metadata.severity.clone());
                    let source = vuln_metadata.source();

                    // Advisory
                    let mut advisory = Advisories::new(vuln.id.clone(), source, severity.clone());
                    debug!("Advisory: {:?}", advisory);
                    advisory.fetch_or_create(connection).await?;
                    advisory.fetch_metadata(connection).await?;

                    // Description
                    if advisory
                        .get_metadata(connection, "description")
                        .await?
                        .is_none()
                    {
                        if !vuln_metadata.description.is_empty() {
                            advisory
                                .add_metadata(
                                    connection,
                                    "description",
                                    vuln_metadata.description.clone(),
                                )
                                .await?;
                        }
                    }
                    if advisory
                        .get_metadata(connection, "description")
                        .await?
                        .is_none()
                    {
                        if !vuln_metadata.description.is_empty() {
                            advisory
                                .add_metadata(
                                    connection,
                                    "description",
                                    vuln_metadata.description.clone(),
                                )
                                .await?;
                        }
                    }
                    if let Some(cvss) = vuln_metadata.cvss {
                        advisory
                            .add_metadata(connection, "cvss", cvss.to_string())
                            .await?;
                    }
                    if let Some(link) = vuln_metadata.urls {
                        advisory.add_metadata(connection, "urls", link).await?;
                    } else {
                        match advisory.source {
                            AdvisorySource::NationalVulnerabilityDatabase => {
                                advisory
                                    .add_metadata(
                                        connection,
                                        "urls",
                                        format!(
                                            "https://nvd.nist.gov/vuln/detail/{}",
                                            vuln_metadata.id
                                        ),
                                    )
                                    .await?
                            }
                            AdvisorySource::GitHubAdvisoryDatabase => {
                                advisory
                                    .add_metadata(
                                        connection,
                                        "urls",
                                        format!(
                                            "https://github.com/advisories/{}",
                                            vuln_metadata.id
                                        ),
                                    )
                                    .await?
                            }
                            _ => {}
                        }
                    }

                    advisory
                } else {
                    debug!("No metadata found for vulnerability, creating generic advisory");
                    let mut advisory =
                        Advisories::new(vuln.id.clone(), AdvisorySource::Anchore, severity.clone());
                    advisory.fetch_or_create(connection).await?;

                    advisory
                };
                advisory
                    .add_metadata(connection, "data.source", "GrypeDB".to_string())
                    .await?;

                let mut alert =
                    Alerts::new(vuln.id.clone(), snapshot.id, dependency.id, advisory.id);
                alert.find_or_create(connection).await?;
                debug!("Created Alert: {}", alert.id);

                *summary.entry(severity).or_insert(0) += 1;

                results.push(alert);
            }
        }

        snapshot.calculate_alerts(connection, &summary).await?;

        Ok(results)
    }
}
