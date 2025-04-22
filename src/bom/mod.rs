//! # Konarr SBOM Module

pub mod cyclonedx;
pub mod sbom;

use sha2::Digest;
use std::path::PathBuf;

pub use sbom::BillOfMaterials;

use crate::KonarrError;
use crate::models::{Component, ComponentVersion, Dependencies, Projects};

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

/// Software Bill of Materials Builder Trait
pub trait BillOfMaterialsBuilder {
    /// Create a new BillOfMaterials
    fn new() -> Self;

    /// Set the Project for the BillOfMaterials (container, etc.)
    fn add_project(&mut self, project: &Projects) -> Result<(), crate::KonarrError>;

    /// Add Dependencies to the BillOfMaterials
    fn add_dependency(&mut self, dep: &Dependencies) -> Result<(), crate::KonarrError> {
        self.add_component(&dep.component_id.data, &dep.component_version_id.data)
    }

    /// Add a list of dependencies to the BillOfMaterials
    fn add_dependencies(
        &mut self,
        dependencies: &Vec<Dependencies>,
    ) -> Result<(), crate::KonarrError> {
        for dep in dependencies.iter() {
            self.add_component(&dep.component_id.data, &dep.component_version_id.data)?;
        }
        Ok(())
    }

    /// Add a component to the BillOfMaterials
    fn add_component(
        &mut self,
        component: &Component,
        version: &ComponentVersion,
    ) -> Result<(), crate::KonarrError>;

    /// Add a list of components to the BillOfMaterials
    fn add_components(&mut self, component: &Vec<Component>) -> Result<(), crate::KonarrError> {
        for comp in component.iter() {
            self.add_component(comp, &ComponentVersion::default())?;
        }
        Ok(())
    }

    /// Build and finalize the BillOfMaterials
    fn output(&self) -> Result<Vec<u8>, crate::KonarrError>;
}

/// Parsers
#[allow(non_camel_case_types)]
pub enum Parsers {
    /// CycloneDX v1.5
    CycloneDX_v1_5,
    /// CycloneDX v1.6
    CycloneDX_v1_6,
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

/// Builders
#[allow(non_camel_case_types)]
pub enum Builders {
    /// CycloneDX v1.6
    CycloneDX_v1_6,
}
