//! # Konarr SBOM Module

pub mod cyclonedx;
pub mod sbom;

use sha2::Digest;
use std::path::PathBuf;

pub use sbom::BillOfMaterials;

use crate::KonarrError;

/// SBOM Parser Trait
pub trait BomParser {
    /// Parse data from bytes
    fn parse(data: &[u8]) -> Result<BillOfMaterials, crate::KonarrError>;
    /// Parse SBOM from file path
    fn parse_path(path: PathBuf) -> Result<BillOfMaterials, crate::KonarrError> {
        let data = std::fs::read(path.clone())?;
        Self::parse(&data)
    }
}

/// Parsers
#[allow(non_camel_case_types)]
pub enum Parsers {
    /// CycloneDX v1.5
    CycloneDX_v1_5,
}

impl BomParser for Parsers {
    fn parse(data: &[u8]) -> Result<BillOfMaterials, crate::KonarrError> {
        let mut hasher = sha2::Sha256::new();
        hasher.update(data);
        let sha = hasher.finalize();

        // CycloneDX
        if let Ok(mut sbom) = cyclonedx::CycloneDx::parse(data) {
            sbom.sha = format!("{:x}", sha);
            Ok(sbom)
        } else {
            Err(KonarrError::ParseSBOM("Failed to parse SBOM".to_string()))
        }
    }
}
