//! CycloneDX 1.5 spec implementation

use log::warn;
use serde::{Deserialize, Serialize};

use crate::bom::{
    BillOfMaterials, BomParser,
    sbom::{BomComponent, BomComponentType, BomTool, BomType, Container},
};

/// CycloneDX SBOM v1.5
#[derive(Debug, Serialize, Deserialize)]
pub struct Bom {
    #[serde(rename = "$schema")]
    pub(crate) schema: Option<String>,
    #[serde(rename = "bomFormat")]
    pub(crate) bom_format: Option<String>,

    #[serde(rename = "specVersion")]
    pub(crate) spec_version: String,

    pub(crate) metadata: Option<Metadata>,

    pub(crate) components: Option<Vec<Component>>,
}

impl BomParser for Bom {
    fn parse(data: &[u8]) -> Result<BillOfMaterials, crate::KonarrError> {
        // Parse JSON data
        let spec_v1_5: Bom = serde_json::from_slice(data)?;

        if spec_v1_5.spec_version != "1.5" {
            return Err(crate::KonarrError::ParseSBOM(
                "Invalid CycloneDX version".to_string(),
            ));
        }

        Ok(spec_v1_5.into())
    }

    fn parse_path(path: std::path::PathBuf) -> Result<BillOfMaterials, crate::KonarrError> {
        // Load JSON file
        let reader = std::fs::File::open(path)?;
        // Parse JSON file
        let spec_v1_5: Bom = serde_json::from_reader(reader)?;

        Ok(spec_v1_5.into())
    }
}

impl From<Bom> for BillOfMaterials {
    fn from(value: Bom) -> Self {
        let mut sbom = BillOfMaterials::new(BomType::CycloneDX_1_5, value.spec_version);

        if let Some(metadata) = value.metadata {
            if let Some(comp) = metadata.component {
                sbom.container = Container {
                    image: comp.name,
                    version: comp.version,
                    ..Container::default()
                };
            }
            if let Some(timestamp) = metadata.timestamp {
                sbom.timestamp = timestamp;
            }
            if let Some(tools) = metadata.tools {
                for tool in tools.components.iter() {
                    sbom.tools.push(BomTool {
                        name: tool.name.clone().unwrap_or_default(),
                        version: tool.version.clone().unwrap_or_default(),
                    });
                }
            }
        }

        if let Some(components) = value.components {
            for comp in components.iter() {
                let purl: String = if let Some(purl) = comp.purl.as_ref() {
                    purl.to_string()
                } else if let Some(name) = comp.name.as_ref() {
                    // HACK: This is really not the right way to do this but 🤷
                    format!("pkg:deb/{}", name)
                } else {
                    warn!("No purl or name found for component");
                    continue;
                };

                let mut bom_comp = BomComponent::from_purl(purl);

                // check if the bom_comp is already in the list
                if sbom.components.contains(&bom_comp) {
                    continue;
                }

                if let Some(typ) = comp.comp_type.as_ref() {
                    bom_comp.comp_type = BomComponentType::from(typ.to_string());
                }

                sbom.components.push(bom_comp);
            }
        }

        sbom
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Metadata {
    pub(crate) timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Creation Tools
    pub(crate) tools: Option<Tools>,
    /// Main Component
    pub(crate) component: Option<Component>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Tools {
    pub(crate) components: Vec<Component>,
    pub(crate) services: Option<Vec<ToolService>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Component {
    /// TODO: This can only be a set of known values
    #[serde(rename = "type")]
    pub(crate) comp_type: Option<String>,

    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,

    pub(crate) purl: Option<String>,

    pub(crate) author: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ToolService {
    pub(crate) vendor: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
}
