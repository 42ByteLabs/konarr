//! CycloneDX BOM (Bill of Materials) support.

pub mod spec_v1_5;

use super::{BillOfMaterials, BomParser};
use spec_v1_5::Bom as Bom_v1_5;

/// CycloneDX SBOM Parser
pub struct CycloneDx;

impl BomParser for CycloneDx {
    fn parse(data: &[u8]) -> Result<BillOfMaterials, crate::KonarrError> {
        if let Ok(sbom) = Bom_v1_5::parse(data) {
            Ok(sbom.into())
        } else {
            Err(crate::KonarrError::ParseSBOM(
                "Failed to parse SBOM".to_string(),
            ))
        }
    }
}
