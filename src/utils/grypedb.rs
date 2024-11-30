//! # Grype Database
#![allow(missing_docs)]
use std::{collections::HashMap, path::PathBuf};

use chrono::Timelike;
use geekorm::prelude::*;
use log::{debug, error, trace, warn};
use semver::Version;
use sha2::Digest;
use url::Url;

use crate::KonarrError;

/// Grype Database
pub struct GrypeDatabase;

impl GrypeDatabase {
    /// Create a connection to the Grype database
    ///
    /// Path can be a directory (with vulnerability.db) or the database file
    pub async fn connect(path: &PathBuf) -> Result<libsql::Connection, KonarrError> {
        let db = if path.is_dir() {
            let fpath = path.join("vulnerability.db");
            libsql::Builder::new_local(fpath).build().await?
        } else {
            libsql::Builder::new_local(path).build().await?
        };
        Ok(db.connect()?)
    }

    /// Sync the Grype database
    ///
    /// The path is the directory where the Grype database is stored
    pub async fn sync(path: &PathBuf) -> Result<(), KonarrError> {
        debug!("Syncing Grype DB");
        let dbpath = path.join("vulnerability.db");
        let archive_path = path.join("vulnerability.tar.gz");

        // Fetch the latest Grype database listing
        let latest = GrypeDatabase::latest().await?;
        let latest_build = latest.built.with_nanosecond(0).unwrap();

        if !path.exists() || !dbpath.exists() {
            if let Some(_) = path.extension() {
                return Err(KonarrError::UnknownError(
                    "Grype path is a file, not a directory".into(),
                ));
            }

            debug!("Grype DB does not exist, downloading latest now");
            std::fs::create_dir_all(path)?;

            debug!("Downloading Grype DB with build: {}", latest.url);
            GrypeDatabase::download(&archive_path, &latest.url).await?;

            if !GrypeDatabase::verify(&archive_path, &latest.checksum)? {
                error!("Checksum verification failed, security risk!");
                return Err(KonarrError::UnknownError(
                    "Checksum verification failed".into(),
                ));
            }

            GrypeDatabase::unarchive(&archive_path)?;
            debug!("Grype DB created and ready to use");
        }

        // Open the Grype database and fetch the db ID metadata
        let grype_db = GrypeDatabase::connect(&dbpath).await?;
        let grype = GrypeDatabase::fetch_grype(&grype_db).await?;
        let build_timestamp = grype.build_timestamp.with_nanosecond(0).unwrap();

        debug!("Grype DB build time: {}", build_timestamp);
        debug!("Latest Grype DB build time: {}", latest_build);

        if latest_build > build_timestamp {
            debug!("Latest Grype DB URL: {}", latest.url);
            GrypeDatabase::download(&path, &latest.url).await?;
            GrypeDatabase::unarchive(&archive_path)?;
        } else {
            debug!("Grype DB is up to date");
        }

        // Clean up the archive
        if archive_path.exists() {
            debug!("Removing Grype DB archive");
            std::fs::remove_file(&archive_path)?;
        }
        Ok(())
    }

    /// Get the Grype database listings
    pub async fn listings() -> Result<GrypeListingResponse, KonarrError> {
        reqwest::get("https://toolbox-data.anchore.io/grype/databases/listing.json")
            .await?
            .json::<GrypeListingResponse>()
            .await
            .map_err(KonarrError::from)
    }

    /// Get the latest Grype database entry from the listings
    pub async fn latest() -> Result<GrypeDatabaseEntry, KonarrError> {
        let response = Self::listings().await?;
        let latest = response
            .latest()
            .ok_or(KonarrError::UnknownError("No latest entry".into()))?;
        assert_eq!(latest.version, 5);
        Ok(latest.clone())
    }
    /// Verify the checksum of the Grype archive file against the provided checksum
    ///
    /// Checksum is the SHA256 checksum provided by the Grype database listing
    ///
    /// Security: We validate the checksum to ensure the Grype database is not tampered with
    fn verify(path: &PathBuf, checksum: &str) -> Result<bool, KonarrError> {
        // Decode the checksum from hex (remove the sha256: prefix)
        let checksum_decode = hex::decode(&checksum[7..])
            .map_err(|_| KonarrError::UnknownError("Unable to decode checksum".into()))?;
        // Generate the SHA256 checksum of the file
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut hasher = sha2::Sha256::new();
        std::io::copy(&mut reader, &mut hasher)?;
        let result = hasher.finalize();

        debug!("GrypeDB Checksum - {} :: {}", hex::encode(result), checksum);
        // Compare the checksums
        Ok(checksum_decode == result.as_slice())
    }

    /// Download a Grype Database archive
    pub async fn download(path: &PathBuf, url: &Url) -> Result<(), KonarrError> {
        debug!("Downloading Grype DB from: {}", url);
        let path_archive = path.join("vulnerability.tar.gz");

        let response = reqwest::get(url.clone()).await?;
        let bytes = response.bytes().await?;

        debug!("Saving to: {:?}", path);
        tokio::fs::write(&path_archive, bytes).await?;

        Ok(())
    }

    /// Unarchive the Grype database tar.gz
    ///
    /// Security: We trust the Grype database to not contain malicious files
    fn unarchive(path: &PathBuf) -> Result<(), KonarrError> {
        if !path.exists() {
            return Err(KonarrError::UnknownError("Archive does not exist".into()));
        }
        if !path.is_file() {
            return Err(KonarrError::UnknownError("Archive is not a file".into()));
        }

        debug!("Unarchiving Grype DB to: {:?}", path.parent().unwrap());
        let tar_gz = std::fs::File::open(path)?;
        let tar = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(path.parent().unwrap())?;

        debug!("Grype DB unarchived");

        Ok(())
    }

    /// Load the Grype database
    pub async fn fetch_grype<'a, T>(connection: &'a T) -> Result<GrypeId, KonarrError>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        Ok(GrypeId::query_first(connection, GrypeId::query_select().limit(1).build()?).await?)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrypeListingResponse {
    pub available: HashMap<u32, Vec<GrypeDatabaseEntry>>,
}

impl GrypeListingResponse {
    pub fn latest(&self) -> Option<&GrypeDatabaseEntry> {
        self.available
            .values()
            .flatten()
            .max_by_key(|entry| entry.version)
    }
}

#[cfg(feature = "models")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrypeDatabaseEntry {
    pub built: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
    pub url: Url,
    pub version: i32,
}

/// Grype Database ID table
#[cfg(feature = "models")]
#[derive(Table, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[geekorm(rename = "id")]
pub struct GrypeId {
    #[geekorm(primary_key)]
    pub build_timestamp: chrono::DateTime<chrono::Utc>,
    pub schema_version: i32,
}

#[derive(Table, Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "models")]
#[geekorm(rename = "vulnerability")]
pub struct GrypeVulnerability {
    #[geekorm(primary_key)]
    pub pk: PrimaryKey<i32>,
    #[geekorm(unique)]
    pub id: String,
    pub package_name: String,
    pub namespace: String,
    pub package_qualifiers: Option<String>,
    pub version_constraint: String,
    pub version_format: String,
    pub cpes: Option<String>,
    pub related_vulnerabilities: Option<String>,
    pub fixed_in_versions: Option<String>,
    pub fix_state: String,
    pub advisories: Option<String>,
}

#[cfg(feature = "models")]
impl GrypeVulnerability {
    pub async fn find_vulnerabilities<'a, T>(
        connection: &'a T,
        component: &crate::models::Component,
        component_version: &crate::models::ComponentVersion,
    ) -> Result<Vec<GrypeVulnerability>, KonarrError>
    where
        T: geekorm::GeekConnection<Connection = T> + 'a,
    {
        if component_version.version.is_empty() {
            warn!("Component version is empty, skipping Grype check");
            return Ok(vec![]);
        }
        if component_version.version.as_str() == "0.0.0" {
            warn!("Unsure what the version of the package is");
            return Ok(vec![]);
        }
        let version = if let Ok(v) = Version::parse(component_version.version.as_str()) {
            v
        } else {
            warn!("Unable to parse version: {}", component_version.version);
            return Ok(vec![]);
        };
        let mut results: Vec<GrypeVulnerability> = vec![];

        // TODO: Manager?
        let vulns =
            GrypeVulnerability::fetch_by_package_name(connection, component.name.clone()).await?;

        debug!(
            "Found {} vulns for package: {}",
            vulns.len(),
            component.name
        );

        for vuln in vulns.iter() {
            if vuln.version_constraint.is_empty() {
                continue;
            }
            if let Ok(versions) = semver::VersionReq::parse(vuln.version_constraint.as_str()) {
                if versions.matches(&version) {
                    if results.iter().any(|v| v.id == vuln.id) {
                        continue;
                    }

                    debug!("Vuln matches version: {}", version);
                    results.push(vuln.clone());
                }
            } else {
                trace!("Unable to parse version req: {}", vuln.version_constraint);
            }
        }

        Ok(results)
    }
}

#[cfg(feature = "models")]
#[derive(Table, Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[geekorm(rename = "vulnerability_metadata")]
pub struct GrypeVulnerabilityMetadata {
    #[geekorm(primary_key, unique)]
    pub id: PrimaryKey<String>,
    pub namespace: String,
    pub data_source: String,
    pub record_source: String,
    pub severity: String,
    pub urls: Option<String>,
    pub description: String,
    pub cvss: Option<String>,
}
