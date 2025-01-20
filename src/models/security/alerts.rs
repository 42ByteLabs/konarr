//! # Security alerts table

use geekorm::prelude::*;
use log::debug;

use super::{advisories::AdvisoriesMetadata, SecuritySeverity};
use crate::{
    bom::sbom::BomVulnerability,
    models::{
        security::{Advisories, AdvisorySource},
        Component, Dependencies, Snapshot,
    },
    KonarrError,
};

/// Security state
#[derive(Data, Debug, Clone, Default, PartialEq)]
pub enum SecurityState {
    /// Vulnerable state
    #[default]
    Vulnerable,
    /// Secure state
    Secure,
    /// Unfixable state
    Unfixable,
}

/// Security alerts table
#[derive(Table, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Alerts {
    /// Primary key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Alert name (CVE, GHSA, etc)
    #[geekorm(searchable)]
    pub name: String,

    /// Security state
    #[geekorm(new = "SecurityState::Vulnerable")]
    pub state: SecurityState,

    /// Affected Snapshot ID
    #[geekorm(foreign_key = "Snapshot.id")]
    pub snapshot_id: ForeignKey<i32, Snapshot>,

    /// Foreign key to the dependencies table
    #[geekorm(foreign_key = "Dependencies.id")]
    pub dependency_id: ForeignKey<i32, Dependencies>,

    /// Foreign key to the advisories table
    #[geekorm(foreign_key = "Advisories.id")]
    pub advisory_id: ForeignKey<i32, Advisories>,

    /// Metadata
    #[serde(skip)]
    #[geekorm(skip)]
    pub metadata: Vec<AdvisoriesMetadata>,

    /// Creation date
    #[geekorm(new = "chrono::Utc::now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated datte
    #[geekorm(new = "chrono::Utc::now()")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Alerts {
    /// Find or create an alert
    pub async fn find_or_create<'a, T>(&mut self, connection: &'a T) -> Result<(), geekorm::Error>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        let item = Self::query_first(
            connection,
            Self::query_select()
                .where_eq("name", self.name.clone())
                .and()
                .where_eq("snapshot_id", self.snapshot_id.clone())
                .and()
                .where_eq("dependency_id", self.dependency_id.clone())
                .and()
                .where_eq("advisory_id", self.advisory_id.clone())
                .build()?,
        )
        .await;

        match item {
            Ok(alert) => {
                self.id = alert.id;
            }
            _ => {
                self.save(connection).await?;
            }
        }

        Ok(())
    }

    /// Filter alerts by severity
    pub async fn filter_severity<'a, T>(
        connection: &'a T,
        severity: SecuritySeverity,
        page: &Page,
    ) -> Result<Vec<Self>, geekorm::Error>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        let mut alerts = Alerts::query(
            connection,
            Alerts::query_select()
                .join(Advisories::table())
                .where_eq("Advisories.severity", severity)
                .page(page)
                .build()?,
        )
        .await?;
        for alert in alerts.iter_mut() {
            alert.fetch(connection).await?;
            alert.fetch_metadata(connection).await?;
        }
        Ok(alerts)
    }

    /// Count Vulnerable alerts
    pub async fn count_vulnerable<'a, T>(connection: &'a T) -> Result<u32, geekorm::Error>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        Ok(Self::row_count(
            connection,
            Self::query_count()
                .where_eq("state", SecurityState::Vulnerable)
                .build()?,
        )
        .await? as u32)
    }

    /// Close An Alert
    pub async fn close<'a, T>(&mut self, connection: &'a T) -> Result<(), geekorm::Error>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        self.state = SecurityState::Secure;
        self.update(connection).await
    }

    /// Fetch metadata for the security advisory
    pub async fn fetch_metadata<'a, T>(&mut self, connection: &'a T) -> Result<(), geekorm::Error>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        self.metadata =
            AdvisoriesMetadata::fetch_by_id(connection, self.advisory_id.data.name.clone()).await?;
        Ok(())
    }

    /// Fetch the severity of the alert
    pub async fn fetch_severity<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<SecuritySeverity, KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let advisory = self.fetch_advisory_id(connection).await?;
        Ok(advisory.severity)
    }

    /// Create an Alert from a Bill of Materials Vulnerability
    pub async fn from_bom_vulnerability<'a, T>(
        connection: &'a T,
        snapshot: &Snapshot,
        vulnerability: &BomVulnerability,
    ) -> Result<Vec<Self>, KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut alerts = Vec::new();

        for affected in &vulnerability.components {
            let (mut component, _) = Component::from_purl(affected.purl.clone())?;
            component.find_or_create(connection).await?;
            debug!("Alert Component: {:?}", component);

            let dependency = match Dependencies::fetch_dependency_by_snapshot(
                connection,
                snapshot.id,
                component.id,
            )
            .await
            {
                Ok(dep) => dep,
                Err(_) => {
                    log::error!(
                        "Failed to fetch dependency for BOM vulnerability: {}",
                        affected.purl
                    );
                    continue;
                }
            };
            debug!("Alert Dependency: {:?}", dependency);

            let mut advisory = Advisories::new(
                vulnerability.name.clone(),
                AdvisorySource::from(vulnerability.source.clone()),
                SecuritySeverity::from(&vulnerability.severity),
            );
            advisory.fetch_or_create(connection).await?;
            advisory.fetch_metadata(connection).await?;

            if !advisory.has_metadata("description") {
                if let Some(desc) = &vulnerability.description {
                    advisory
                        .add_metadata(connection, "description", desc)
                        .await?;
                }
            }
            if !advisory.has_metadata("url") {
                if let Some(url) = &vulnerability.url {
                    advisory.add_metadata(connection, "url", url).await?;
                }
            }
            if !advisory.has_metadata("source") {
                advisory
                    .add_metadata(connection, "source", &vulnerability.source)
                    .await?;
            }

            // TODO: Metadata for the advisory
            debug!("Alert Advisory: {:?}", advisory);

            let mut alert = Alerts::new(
                vulnerability.name.clone(),
                snapshot.id,
                dependency.id,
                advisory.id,
            );
            alert.find_or_create(connection).await?;
            debug!("Alert: {:?}", alert);
            alerts.push(alert);
        }

        Ok(alerts)
    }

    /// Get the description of the alert (if available in the metadata)
    pub fn description(&self) -> Option<String> {
        self.advisory_id
            .data
            .metadata
            .iter()
            .find(|m| m.key == "description")
            .map(|m| m.value.clone())
    }

    /// Get the URL of the alert (if available in the metadata)
    pub fn url(&self) -> Option<String> {
        self.advisory_id
            .data
            .metadata
            .iter()
            .find(|m| m.key == "url")
            .map(|m| m.value.clone())
    }
}

impl From<Option<String>> for SecurityState {
    fn from(state: Option<String>) -> Self {
        match state {
            Some(value) => SecurityState::from(value),
            None => SecurityState::Vulnerable,
        }
    }
}
