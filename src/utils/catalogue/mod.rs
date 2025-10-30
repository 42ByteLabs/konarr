//! # Catalogue
use std::collections::HashMap;

use crate::models::{Component, ComponentManager, ComponentType};

const CATALOGUE: &str = include_str!("data.yml");

/// Catalogger
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Catalogue {
    /// Aliases for components
    aliases: HashMap<String, String>,
    /// Component Mapping Table
    catalogue: HashMap<String, ComponentType>,
}

impl Catalogue {
    /// New Catalogue
    pub fn new() -> Self {
        #[cfg(debug_assertions)]
        let data: Self = serde_yaml::from_str(CATALOGUE).expect("Failed to load catalogue data");
        #[cfg(not(debug_assertions))]
        let data: Self = serde_yaml::from_str(CATALOGUE).unwrap_or_default();

        log::debug!("Loaded Catalogue Data: {}", data.catalogue.len());
        data
    }

    /// Catalogue the component
    ///
    /// Match manager -> type
    pub fn catalogue(&self, component: &mut Component) -> Result<bool, crate::KonarrError> {
        let comp_purl = component.purl();

        // Exact PURL matching
        if let Some(comp) = self.catalogue.get(&comp_purl) {
            if component.component_type != *comp {
                log::debug!("Updating component type for: {}", comp_purl);
                component.component_type = comp.clone();
                return Ok(true);
            }
        } else {
            let wildcards = [format!("pkg:*/{}", component.name),
                format!("pkg:{}/*", component.manager)];

            for wildcard in wildcards.iter() {
                if let Some(comp) = self.catalogue.get(wildcard) {
                    if component.component_type != *comp {
                        log::debug!("Updating component type for: {}", comp_purl);
                        component.component_type = comp.clone();
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Set the component type based on the name of the component
    ///
    /// This is a simple and quick method to set the component type based on the name of the
    /// component. It is not exhaustive and should be used in conjunction with the `catalogue`
    /// function to ensure the most accurate component type is set.
    pub fn catalogue_old(component: &mut Component) -> Result<(), crate::KonarrError> {
        if component.manager == ComponentManager::Apk || component.manager == ComponentManager::Deb
        {
            // We don't care about the namespace for these package managers

            match component.name.to_lowercase().as_str() {
                // Operating Systems
                "alpine" | "alpine-linux" | "debian" | "debian-linux" | "ubuntu"
                | "ubuntu-linux" | "redhat" | "fedora" | "centos" | "centos-linux" | "arch"
                | "arch-linux" => {
                    component.component_type = ComponentType::OperatingSystem;
                }
                // Programming Languages (compilers / interpreters / runtimes)
                "python" | "python3" | "node" | "nodejs" | "ruby" | "rustc" | "rust" | "go"
                | "java" | "javac" | "kotlinc" | "gcc" | "g++" | "gpp" | "dotnet" | "csharp"
                | "c" | "cpp" | "php83" | "perl" | "bash" | "sh" => {
                    component.component_type = ComponentType::ProgrammingLanguage;
                }
                // Package Managers
                "apk" | "apk-tools" | "deb" | "dpkg" | "rpm" | "cargo" | "npm" | "pip"
                | "composer" | "maven" | "nuget" | "gradle" | "gem" => {
                    component.component_type = ComponentType::PackageManager;
                }
                // Cryptography Libraries
                "openssl" | "libssl" | "libssl3" | "libcrypto" | "libcrypto3" | "libssl-dev"
                | "libcrypto-dev" | "argon2-libs" | "ssl_client" => {
                    component.component_type = ComponentType::CryptographyLibrary;
                }
                // Databases
                "mysql" | "mariadb" | "postgresql" | "sqlite" | "mongodb" | "redis"
                | "cassandra" => {
                    component.component_type = ComponentType::Database;
                }
                // Applications
                "curl" | "wget" | "git" | "grep" | "jq" | "nginx" => {
                    component.component_type = ComponentType::Application;
                }
                "apr" | "apr-util" | "busybox" | "busybox-binsh" => {
                    component.component_type = ComponentType::OperatingEnvironment;
                }
                _ => {
                    component.component_type = ComponentType::Library;
                }
            }
        } else if component.manager == ComponentManager::Golang {
            // Official Go namespaces
            match component.purl().as_str() {
                "pkg:golang/cloud.google.com/go" => {
                    component.component_type = ComponentType::Library;
                }
                "pkg:golang.org/x/crypto" => {
                    component.component_type = ComponentType::CryptographyLibrary;
                }
                _ => {
                    component.component_type = ComponentType::Library;
                }
            }
        } else if component.manager == ComponentManager::Generic {
            // Generic manager are typically applications
            component.component_type = ComponentType::Application;
        } else {
            // Default to library
            component.component_type = ComponentType::Library;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcards() {
        let mut catalogue = Catalogue::default();
        catalogue.catalogue.insert(
            "pkg:*/openssl".to_string(),
            ComponentType::CryptographyLibrary,
        );

        let mut data = vec![
            Component::from_purl("pkg:deb/debian/openssl").unwrap(),
            Component::from_purl("pkg:apk/alpine/openssl").unwrap(),
            Component::from_purl("pkg:rpm/fedora/openssl").unwrap(),
            Component::from_purl("pkg:generic/openssl").unwrap(),
            Component::from_purl("pkg:conan/openssl").unwrap(),
            Component::from_purl("pkg:conan/openssl.org/openssl").unwrap(),
            Component::from_purl("pkg:alpm/arch/openssl").unwrap(),
        ];

        for (comp, _ver) in data.iter_mut() {
            let _output = catalogue.catalogue(comp).unwrap();
            assert_eq!(comp.component_type, ComponentType::CryptographyLibrary);
        }
    }

    #[test]
    fn test_catalogue() {
        let catalogue = Catalogue::new();

        let mut data = vec![
            (
                Component::from_purl("pkg:apk/alpine").unwrap(),
                ComponentType::OperatingSystem,
            ),
            (
                Component::from_purl("pkg:deb/debian/openssl").unwrap(),
                ComponentType::CryptographyLibrary,
            ),
            (
                Component::from_purl("pkg:deb/debian").unwrap(),
                ComponentType::OperatingSystem,
            ),
            (
                Component::from_purl("pkg:apk/alpine/python3").unwrap(),
                ComponentType::ProgrammingLanguage,
            ),
            (
                Component::from_purl("pkg:apk/alpine/openjdk8").unwrap(),
                ComponentType::ProgrammingLanguage,
            ),
        ];

        for ((comp, _ver), expected) in data.iter_mut() {
            let _output = catalogue.catalogue(comp).unwrap();
            assert_eq!(&comp.component_type, expected);
        }
    }
}
