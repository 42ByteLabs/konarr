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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) metadata: Option<Metadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) components: Option<Vec<Component>>,

    #[serde(skip_serializing_if = "Option::is_none")]
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
            schema: Some("http://cyclonedx.org/schema/bom-1.6.schema.json".to_string()),
            bom_format: Some("CycloneDX".to_string()),
            spec_version: "1.6".to_string(),
            metadata: None,
            components: Some(vec![]),
            vulnerabilities: None,
        }
    }

    fn add_project(&mut self, project: &crate::models::Projects) -> Result<(), crate::KonarrError> {
        let tool: Option<Tools> = if let Some(snapshot) = project.snapshots.last() {
            let name = snapshot
                .find_metadata("bom.tool.name")
                .map(|v| v.as_string());
            let version = snapshot
                .find_metadata("bom.tool.version")
                .map(|v| v.as_string());

            Some(Tools {
                components: vec![
                    Component {
                        comp_type: Some("application".to_string()),
                        name: Some("konarr".to_string()),
                        version: Some(crate::KONARR_VERSION.to_string()),
                        ..Default::default()
                    },
                    Component {
                        comp_type: Some("application".to_string()),
                        name: name,
                        version: version,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            })
        } else {
            None
        };

        if let Some(metadata) = self.metadata.as_mut() {
            metadata.timestamp = Some(project.created_at);
            metadata.tools = tool;

            metadata.component = Some(Component {
                comp_type: Some("container".to_string()),
                name: Some(project.name.clone()),
                version: project.version(),
                ..Default::default()
            });
        } else {
            self.metadata = Some(Metadata {
                timestamp: Some(chrono::Utc::now()),
                tools: tool,
                component: Some(Component {
                    comp_type: Some("container".to_string()),
                    name: Some(project.name.clone()),
                    version: project.version(),
                    ..Default::default()
                }),
            });
        }

        Ok(())
    }

    fn add_dependency(
        &mut self,
        dep: &crate::models::Dependencies,
    ) -> Result<(), crate::KonarrError> {
        if let Some(components) = self.components.as_mut() {
            components.push(Component::from(dep));
        }

        Ok(())
    }

    fn add_component(
        &mut self,
        component: &crate::models::Component,
        version: &crate::models::ComponentVersion,
    ) -> Result<(), crate::KonarrError> {
        if let Some(comps) = self.components.as_mut() {
            comps.push(Component {
                comp_type: Some(comptype_to_string(&component.component_type)),
                name: Some(component.name.to_string()),
                version: Some(version.version()?.to_string()),
                purl: Some(component.purl()),
                ..Default::default()
            });
        }
        Ok(())
    }

    fn output(&self) -> Result<Vec<u8>, crate::KonarrError> {
        Ok(serde_json::to_vec(&self)?)
    }
}

/// Convert the component type to a string
///
/// Values:
/// "framework",
/// "library",
/// "container",
/// "platform",
/// "operating-system",
/// "device",
/// "device-driver",
/// "firmware",
/// "file",
/// "machine-learning-model",
/// "data",
/// "cryptographic-asset"
fn comptype_to_string(typ: &crate::models::ComponentType) -> String {
    match typ {
        crate::models::ComponentType::Library
        | crate::models::ComponentType::Framework
        | crate::models::ComponentType::OperatingEnvironment => "library".to_string(),
        crate::models::ComponentType::Container => "container".to_string(),
        crate::models::ComponentType::CryptographyLibrary => "cryptographic-asset".to_string(),
        crate::models::ComponentType::Application
        | crate::models::ComponentType::ProgrammingLanguage => "platform".to_string(),
        crate::models::ComponentType::Firmware => "firmware".to_string(),
        _ => "library".to_string(),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Metadata {
    pub(crate) timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Creation Tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tools: Option<Tools>,
    /// Main Component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) component: Option<Component>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Tools {
    pub(crate) components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) services: Option<Vec<ToolService>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct Component {
    /// TODO: This can only be a set of known values
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub(crate) comp_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) purl: Option<String>,

    /// Component Author (deprecated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) author: Option<String>,
}

impl From<&crate::models::Dependencies> for Component {
    fn from(value: &crate::models::Dependencies) -> Self {
        let comptype = value.component_type();
        Component {
            comp_type: Some(comptype_to_string(&comptype)),
            name: Some(value.name()),
            version: value.version(),
            purl: Some(value.purl()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ToolService {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Vulnerability {
    #[serde(rename = "bom-ref")]
    pub(crate) bom_ref: String,
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<VulnerabilitySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) references: Option<Vec<VulnerabilityRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ratings: Option<Vec<VulnerabilityRating>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
