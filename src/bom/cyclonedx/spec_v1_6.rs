//! CycloneDX 1.6 spec implementation

use log::warn;
use serde::{Deserialize, Serialize};

use crate::bom::{
    BillOfMaterials, BillOfMaterialsBuilder, BomParser,
    sbom::{BomComponent, BomComponentType, BomTool, BomType, BomVulnerability, Container},
};

/// CycloneDX SBOM v1.6
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

    pub(crate) vulnerabilities: Option<Vec<Vulnerability>>,
}

impl BomParser for Bom {
    fn parse(data: &[u8]) -> Result<BillOfMaterials, crate::KonarrError> {
        // Parse JSON data
        let spec_v1_6: Bom = serde_json::from_slice(data)?;

        if spec_v1_6.spec_version != "1.6" {
            return Err(crate::KonarrError::ParseSBOM(
                "Invalid CycloneDX version".to_string(),
            ));
        }

        Ok(spec_v1_6.into())
    }

    fn parse_path(path: std::path::PathBuf) -> Result<BillOfMaterials, crate::KonarrError> {
        // Load JSON file
        let reader = std::fs::File::open(path)?;
        // Parse JSON file
        let spec_v1_6: Bom = serde_json::from_reader(reader)?;

        Ok(spec_v1_6.into())
    }
}

impl From<Bom> for BillOfMaterials {
    fn from(value: Bom) -> Self {
        let mut sbom = BillOfMaterials::new(BomType::CycloneDX_1_6, value.spec_version);

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

        if let Some(vulns) = value.vulnerabilities {
            for vulnerability in vulns.iter() {
                let severity = vulnerability
                    .ratings
                    .as_ref()
                    .map_or("Unknown".to_string(), |r| {
                        r.iter()
                            .max_by_key(|rating| rating.severity.len())
                            .map_or("Unknown".to_string(), |rating| rating.severity.clone())
                    });
                let source = vulnerability
                    .source
                    .as_ref()
                    .map_or("Unknown".to_string(), |s| s.name.clone());

                let mut vuln = BomVulnerability::new(vulnerability.id.clone(), source, severity);

                if let Some(desc) = &vuln.description {
                    vuln.description = Some(desc.clone());
                }
                if let Some(source) = &vulnerability.source {
                    vuln.url = Some(source.url.clone());
                }

                if let Some(affects) = &vulnerability.affects {
                    for v in affects {
                        vuln.components
                            .push(BomComponent::from_purl(v.reference.clone()));
                    }
                }

                sbom.vulnerabilities.push(vuln);
            }
        }

        sbom
    }
}

impl BillOfMaterialsBuilder for Bom {
    fn new() -> Self {
        Self {
            schema: Some("https://cyclonedx.org/schema/bom-1.6.json".to_string()),
            bom_format: Some("CycloneDX".to_string()),
            spec_version: "1.6".to_string(),
            metadata: None,
            components: Some(vec![]),
            vulnerabilities: None,
        }
    }

    fn add_project(&mut self, project: &crate::models::Projects) -> Result<(), crate::KonarrError> {
        if let Some(metadata) = self.metadata.as_mut() {
            metadata.timestamp = Some(project.created_at);
            metadata.component = Some(Component {
                name: Some(project.name.clone()),
                ..Default::default()
            });
        } else {
            self.metadata = Some(Metadata {
                timestamp: Some(chrono::Utc::now()),
                component: Some(Component {
                    name: Some(project.name.clone()),
                    ..Default::default()
                }),
                tools: None,
            });
        }

        Ok(())
    }

    fn add_component(
        &mut self,
        component: &crate::models::Component,
        version: &crate::models::ComponentVersion,
    ) -> Result<(), crate::KonarrError> {
        if let Some(components) = self.components.as_mut() {
            let comp = Component {
                comp_type: Some(component.component_type.to_string()),
                name: Some(component.name.clone()),
                version: Some(version.version.clone()),
                purl: Some(component.purl()),
                ..Default::default()
            };
            components.push(comp);
        }

        Ok(())
    }

    fn output(&self) -> Result<Vec<u8>, crate::KonarrError> {
        Ok(serde_json::to_vec(&self)?)
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

#[derive(Debug, Default, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Vulnerability {
    #[serde(rename = "bom-ref")]
    pub(crate) bom_ref: String,
    pub(crate) id: String,
    pub(crate) source: Option<VulnerabilitySource>,
    pub(crate) references: Option<Vec<VulnerabilityRef>>,
    pub(crate) ratings: Option<Vec<VulnerabilityRating>>,
    pub(crate) description: Option<String>,
    pub(crate) affects: Option<Vec<VulnerabilityCompRef>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VulnerabilitySource {
    pub(crate) name: String,
    pub(crate) url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VulnerabilityRating {
    pub(crate) severity: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VulnerabilityRef {
    pub id: String,
    pub source: VulnerabilitySource,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct VulnerabilityCompRef {
    #[serde(rename = "ref")]
    pub(crate) reference: String,
}
