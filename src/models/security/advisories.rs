//! # Konarr Security Advisories
//!
//!
use geekorm::prelude::*;

use super::SecuritySeverity;

/// Advisory Sources
///
/// The main source for the Advisories is from Anchore
///
/// - https://github.com/anchore/grype?tab=readme-ov-file#grypes-database
///
#[derive(Data, Debug, Clone, Default, PartialEq)]
pub enum AdvisorySource {
    /// Alpine Security DB
    #[geekorm(aliases = "alpine,alpinesecdb")]
    AlpineSecDB,
    /// Amazon Web Services
    #[geekorm(aliases = "amazon,aws,amazonwebservices")]
    AmazonWebServices,
    /// Anchore
    #[geekorm(aliases = "anchore")]
    Anchore,
    /// Chainguard
    #[geekorm(aliases = "chainguard")]
    Chainguard,
    /// Debian
    #[geekorm(aliases = "debian,debian-sec,debian-distro-debian-12")]
    Debian,
    /// GitHub Advisory Database
    #[geekorm(aliases = "github,ghad,githubadvisories")]
    GitHubAdvisoryDatabase,
    /// National Vulnerability Database
    #[geekorm(aliases = "nvd,nationalvulnerabilitydatabase")]
    NationalVulnerabilityDatabase,
    /// Oracle
    #[geekorm(aliases = "oracle,oracleoval")]
    OracleOval,
    /// RedHat
    #[geekorm(aliases = "redhat,redhatsecurity")]
    RedHatSecurity,
    /// SUSE
    #[geekorm(aliases = "suse,suseoval")]
    SuseOval,
    /// Ubuntu
    #[geekorm(aliases = "ubuntu")]
    UbuntuSecurity,
    /// Wolfi
    #[geekorm(aliases = "wolfi")]
    WolfiSecDB,
    /// Custom source of security information
    #[geekorm(aliases = "custom")]
    Custom,
    /// Unknown
    #[default]
    Unknown,
}

/// Security vulnerabilities table
#[derive(Table, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Advisories {
    /// Primary key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,
    /// Advisory Name given by the source (CVEs, GHSA, etc.)
    #[geekorm(unique)]
    pub name: String,
    /// Vulnerability Source
    pub source: AdvisorySource,
    /// Base Severity for the advisory
    pub severity: SecuritySeverity,

    /// Created advisory date
    #[geekorm(new = "chrono::Utc::now()")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated at
    #[geekorm(new = "chrono::Utc::now()")]
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Vulnerability metadata
    #[geekorm(skip)]
    #[serde(skip)]
    pub metadata: Vec<AdvisoriesMetadata>,
}

impl Advisories {
    /// Fetch metadata for the security vulnerability
    pub async fn fetch_metadata<'a, T>(&mut self, connection: &'a T) -> Result<(), geekorm::Error>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        self.metadata = AdvisoriesMetadata::fetch_by_advisory_id(connection, self.id).await?;

        Ok(())
    }

    /// Check if the advisory has metadata (assumes metadata is fetched)
    pub fn has_metadata(&self, key: impl Into<String>) -> bool {
        let key = key.into();
        self.metadata.iter().any(|m| m.key == key)
    }

    /// Add Advisory metadata
    pub async fn add_metadata<'a, T>(
        &mut self,
        connection: &'a T,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), geekorm::Error>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let key = key.into();
        log::debug!("Adding advisory metadata `{}`", key);

        let meta = match AdvisoriesMetadata::query_first(
            connection,
            AdvisoriesMetadata::query_select()
                .where_eq("key", key.clone())
                .and()
                .where_eq("advisory_id", self.id)
                .build()?,
        )
        .await
        {
            Ok(meta) => meta,
            Err(_) => {
                let mut meta = AdvisoriesMetadata::new(key, value.into(), self.id);
                meta.save(connection).await?;
                meta
            }
        };

        self.metadata.push(meta);
        Ok(())
    }

    /// Fetch the metadata by key
    pub async fn get_metadata<'a, T>(
        &mut self,
        connection: &'a T,
        key: impl Into<String>,
    ) -> Result<Option<AdvisoriesMetadata>, geekorm::Error>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        let key = key.into();
        log::debug!("Getting advisory metadata `{}`", key);
        let meta = self.metadata.iter().find(|m| m.key == key);

        if let Some(meta) = meta {
            return Ok(Some(meta.clone()));
        }

        let meta = if let Ok(m) = AdvisoriesMetadata::query_first(
            connection,
            AdvisoriesMetadata::query_select()
                .where_eq("key", key)
                .and()
                .where_eq("advisory_id", self.id)
                .build()?,
        )
        .await
        {
            m
        } else {
            return Ok(None);
        };
        self.metadata.push(meta.clone());
        Ok(Some(meta))
    }
}

/// Security vulnerability metadata table
#[derive(Table, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AdvisoriesMetadata {
    /// Primary key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,
    /// Key of the metadata
    pub key: String,
    /// Value of the metadata
    pub value: String,
    /// Foreign key to the security vulnerabilities table
    #[geekorm(foreign_key = "Advisories.id")]
    pub advisory_id: ForeignKey<i32, Advisories>,
    /// Updated last the metadata
    #[geekorm(new = "chrono::Utc::now()")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
