//! # Grype Database
#![allow(missing_docs)]
#![allow(clippy::needless_question_mark)]

use chrono::Timelike;
use geekorm::{ConnectionManager, prelude::*};
use log::{debug, error, trace, warn};
use semver::Version;
use sha2::Digest;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use url::Url;

use crate::{
    KonarrError,
    bom::{BillOfMaterials, BomParser, Parsers},
    models::security::AdvisorySource,
    tools::{Grype, Tool, ToolConfig},
};

mod matcher;

/// Grype Database
pub struct GrypeDatabase {
    /// Connection to the Grype database
    pub connection: ConnectionManager,
    /// All of the Grype vulnerabilities
    ///
    /// Acts like a cache
    pub vulnerabilities: Vec<GrypeVulnerability>,

    /// Grype tool configuration
    pub tool: ToolConfig,
}

impl GrypeDatabase {
    /// Create a connection to the Grype database
    ///
    /// Path can be a directory (with vulnerability.db) or the database file
    pub async fn connect(path: &Path) -> Result<Self, KonarrError> {
        log::debug!("Connecting to Grype DB at: {}", path.display());
        let db = if path.is_dir() {
            path.join("5").join("vulnerability.db")
        } else {
            path.to_path_buf()
        };

        Ok(Self {
            connection: ConnectionManager::path(db).await?,
            vulnerabilities: Vec::new(),
            tool: Grype::init().await,
        })
    }

    /// Sync the Grype database
    ///
    /// The path is the directory where the Grype database is stored
    pub async fn sync(path: &Path) -> Result<bool, KonarrError> {
        debug!("Syncing Grype DB");
        let dbpath = path.join("5").join("vulnerability.db");

        // Fetch the latest Grype database listing
        let latest = GrypeDatabase::latest().await?;
        debug!("Latest Grype DB: {}", latest.built);
        let latest_build = latest.built.with_nanosecond(0).unwrap();

        if !dbpath.exists() {
            if path.extension().is_some() {
                return Err(KonarrError::UnknownError(
                    "Grype path is a file, not a directory".into(),
                ));
            }

            if let Some(parent) = dbpath.parent() {
                debug!("Creating Grype DB parent directories: {:?}", parent);
                std::fs::create_dir_all(parent)?;
            }

            debug!("Downloading Grype DB with build: {}", latest.url);
            GrypeDatabase::download(path, &latest).await?;
            debug!("Grype DB created and ready to use");
        }

        // Open the Grype database and fetch the db ID metadata
        let grype_db = GrypeDatabase::connect(&dbpath).await?;
        let grype = grype_db.fetch_grype().await?;
        let build_timestamp = grype.build_timestamp.with_nanosecond(0).unwrap();

        debug!("Grype DB build time: {}", build_timestamp);
        debug!("Latest Grype DB build time: {}", latest_build);

        let mut new = false;
        if latest_build > build_timestamp {
            debug!("New Grype DB available, updating...");
            debug!("Latest Grype DB URL: {}", latest.url);
            GrypeDatabase::download(path, &latest).await?;
            new = true;
        } else {
            debug!("Grype DB is up to date");
        }

        Ok(new)
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

    /// Download, verify and unarchive a build of the Grype database
    ///
    /// This is the full process of updating the Grype database
    pub async fn download(path: &Path, build: &GrypeDatabaseEntry) -> Result<(), KonarrError> {
        debug!("Downloading Grype DB from: {}", build.url);
        let path_version = path.join(build.version.to_string());
        if !path_version.exists() {
            std::fs::create_dir_all(&path_version)?;
        }
        debug!("Grype DB Path: {:?}", path_version);

        let archive_path = GrypeDatabase::download_archive(&path_version, &build.url).await?;

        if !GrypeDatabase::verify(&archive_path, &build.checksum)? {
            error!("Checksum verification failed, security risk!");
            return Err(KonarrError::UnknownError(
                "Checksum verification failed".into(),
            ));
        }

        GrypeDatabase::unarchive(&archive_path)?;
        debug!("Grype DB created and ready to use");

        // Clean up the archive
        if archive_path.exists() {
            debug!("Removing Grype DB archive");
            std::fs::remove_file(&archive_path)?;
        }
        Ok(())
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
    async fn download_archive(path: &PathBuf, url: &Url) -> Result<PathBuf, KonarrError> {
        debug!("Downloading Grype DB from: {}", url);
        let path_archive = path.join("vulnerability.tar.gz");

        if path_archive.exists() {
            debug!("Removing existing Grype DB archive");
            std::fs::remove_file(&path_archive)?;
        }

        let response = reqwest::get(url.clone()).await?;
        let bytes = response.bytes().await?;

        debug!("Saving to: {:?}", path);
        tokio::fs::write(&path_archive, bytes).await?;
        debug!("Finished downloading and writing Grype DB");

        Ok(path_archive)
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

    /// Scan a SBOM with Grype
    pub async fn scan_sbom(&self, path: &Path) -> Result<BillOfMaterials, KonarrError> {
        let sbom = format!("sbom:{}", path.display());
        let output = Grype::run(&self.tool, sbom).await?;

        let bom = Parsers::parse(output.as_bytes())?;

        Ok(bom)
    }

    /// Load the Grype database
    pub async fn fetch_grype(&self) -> Result<GrypeId, KonarrError> {
        Ok(GrypeId::query_first(
            &self.connection.acquire().await,
            GrypeId::query_select().limit(1).build()?,
        )
        .await?)
    }

    pub async fn fetch_vulnerabilities(&mut self) -> Result<&Vec<GrypeVulnerability>, KonarrError> {
        if self.vulnerabilities.is_empty() {
            debug!("Loading Grype vulnerabilities");
            self.vulnerabilities = GrypeVulnerability::query(
                &self.connection.acquire().await,
                GrypeVulnerability::query_select().build()?,
            )
            .await?;
            debug!(
                "Loaded {} Grype vulnerabilities",
                self.vulnerabilities.len()
            );
        }
        Ok(&self.vulnerabilities)
    }

    /// Find a vulnerability in the Grype database
    pub fn find_vulnerability(
        &self,
        comp: &crate::models::Component,
        compversion: &crate::models::ComponentVersion,
    ) -> Result<Vec<GrypeVulnerability>, crate::KonarrError> {
        if compversion.version.is_empty() {
            warn!("Component version is empty, skipping Grype check");
            return Ok(vec![]);
        }
        if compversion.version.as_str() == "0.0.0" {
            warn!("Unsure what the version of the package is");
            return Ok(vec![]);
        }

        // TODO: Only semver for now
        let version = if let Ok(v) = Version::parse(compversion.version.as_str()) {
            v
        } else {
            debug!(
                "Unable to parse version `{}` for component `{}`",
                compversion.version, comp.name
            );
            return Ok(vec![]);
        };

        let mut results = vec![];

        // TODO: This is a issue, this has 4million+ entries
        for vuln in self.vulnerabilities.iter() {
            // Skip if no version constraint
            if vuln.version_constraint.is_empty() {
                continue;
            }
            // Name matching
            if vuln.package_name != comp.name {
                continue;
            }

            // Version matching
            if let Ok(versions) = semver::VersionReq::parse(vuln.version_constraint.as_str()) {
                if versions.matches(&version) {
                    results.push(vuln.clone());
                }
            } else {
                trace!("Unable to parse version req: {}", vuln.version_constraint);
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrypeListingResponse {
    pub available: HashMap<u32, Vec<GrypeDatabaseEntry>>,
}

impl GrypeListingResponse {
    /// Get the latest Grype database entry
    ///
    /// This is the latest entry with version 5
    pub fn latest(&self) -> Option<&GrypeDatabaseEntry> {
        self.available
            .iter()
            .find(|e| *e.0 == 5)
            .and_then(|e| e.1.first())
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
#[geekorm(db = "GrypeDatabase", rename = "id")]
pub struct GrypeId {
    #[geekorm(primary_key)]
    pub build_timestamp: chrono::DateTime<chrono::Utc>,
    pub schema_version: i32,
}

/// Grype Vulnerability table
#[derive(Table, Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "models")]
#[geekorm(db = "GrypeDatabase")]
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
    pub fn cpes(&self) -> Vec<String> {
        if let Some(cpes) = &self.cpes {
            serde_json::from_str(cpes).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

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

        // TODO: Only semver for now
        let version = if let Ok(v) = Version::parse(component_version.version.as_str()) {
            v
        } else {
            warn!(
                "Unable to parse version `{}` for component `{}`",
                component_version.version, component.name
            );
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

            // Check against CPEs to check if it's a match
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
#[geekorm(db = "GrypeDatabase")]
pub struct GrypeVulnerabilityMetadata {
    #[geekorm(primary_key)]
    pub id: PrimaryKey<String>,

    pub namespace: String,
    pub data_source: String,
    pub record_source: String,
    pub severity: String,
    pub urls: Option<String>,
    pub description: String,
    pub cvss: Option<String>,
}

impl GrypeVulnerabilityMetadata {
    /// Convert the GrypeDB record source into AdvisorySource
    pub fn source(&self) -> AdvisorySource {
        // TODO: Add all sources
        match self.record_source.as_str() {
            "nvdv2:nvdv2:cves" => AdvisorySource::NationalVulnerabilityDatabase,
            "vulnerabilities:chainguard:rolling" => AdvisorySource::Chainguard,
            "vulnerabilities:wolfi:rolling" => AdvisorySource::WolfiSecDB,
            s if s.starts_with("github:github:") => AdvisorySource::GitHubAdvisoryDatabase,
            s if s.starts_with("vulnerabilities:alpine:") => AdvisorySource::AlpineSecDB,
            s if s.starts_with("vulnerabilities:debian:") => AdvisorySource::Debian,
            s if s.starts_with("vulnerabilities:ubuntu:") => AdvisorySource::UbuntuSecurity,
            s if s.starts_with("vulnerabilities:rhel:") => AdvisorySource::RedHatSecurity,
            // Default to Anchore
            _ => AdvisorySource::Anchore,
        }
    }
}
