//! # Dependency Components Models / Tables

use geekorm::prelude::*;
use log::{debug, info};
use purl::GenericPurl;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use super::ComponentType;

/// Component Model
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct Component {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Component Type (Library, Application, etc)
    #[geekorm(new = "ComponentType::Unknown")]
    pub component_type: ComponentType,
    /// Package Manager (cargo, deb, etc)
    pub manager: ComponentManager,
    /// Package Namespace
    pub namespace: Option<String>,
    /// Package Name
    pub name: String,
}

impl Component {
    /// Initialise Components
    ///
    /// This function will also check and automatically set the ComponentType of existing
    /// components if they are unknown, libraries, or applications.
    pub async fn init<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        debug!("Creating and Initialising Component Table");
        Component::create_table(connection).await?;

        let purls = vec!["pkg:deb/debian", "pkg:apk/alpine"];
        for purl in purls.iter() {
            let (mut comp, _version) = Component::from_purl(purl.to_string()).unwrap();
            comp.find_or_create(connection).await?;
        }

        let mut counter = 0;
        let mut comps = Component::fetch_all(connection).await?;
        debug!("Checking component types for `{}` Components", comps.len());

        for mut comp in comps.iter_mut() {
            match comp.component_type {
                ComponentType::Unknown | ComponentType::Library | ComponentType::Application => {
                    let og_type = comp.component_type.clone();
                    Self::set_purl_comptype(&mut comp);

                    if og_type != comp.component_type {
                        debug!("Updating component_type: {}", comp.component_type);
                        comp.update(connection).await?;
                        counter += 1;
                    }
                }
                _ => {}
            }
        }
        if counter != 0 {
            info!("Updated `{}` component out of `{}`", counter, comps.len());
        }

        Ok(())
    }

    /// Create PURL from Component
    pub fn purl(&self) -> String {
        let mut purl = format!("pkg:{}/", self.manager.to_string().to_lowercase());

        if let Some(namespace) = &self.namespace {
            purl += format!("{}/", namespace).as_str();
        }
        purl += format!("{}", self.name.as_str()).as_str();

        purl
    }

    /// Create Component from Package URL
    pub fn from_purl(
        value: impl Into<String>,
    ) -> Result<(Self, ComponentVersion), crate::KonarrError> {
        let purl = GenericPurl::<String>::from_str(value.into().as_str())
            .map_err(|e| crate::KonarrError::UnknownError(e.to_string()))?;

        let mut component = Component::new(purl.package_type(), purl.name().to_string());
        Self::set_purl_comptype(&mut component);

        if let Some(namespace) = purl.namespace() {
            component.namespace = Some(namespace.to_string());
        }

        let version: ComponentVersion = if let Some(version) = purl.version() {
            let v = version.replace('v', "");
            ComponentVersion::new(component.id, v)
        } else {
            ComponentVersion::new(component.id, "0.0.0".to_string())
        };

        Ok((component, version))
    }

    /// Set the component type based on the name of the component
    ///
    /// TODO: This is a little bit of a hack, but it works for now
    fn set_purl_comptype(component: &mut Component) {
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
            if component.namespace == Some("golang.org/x".to_string())
                || component.namespace == Some("cloud.google.com".to_string())
            {
                if component.name == "go" {
                    component.component_type = ComponentType::ProgrammingLanguage;
                } else if component.name == "crypto" {
                    component.component_type = ComponentType::CryptographyLibrary;
                } else {
                    component.component_type = ComponentType::Library;
                }
            } else {
                component.component_type = ComponentType::Library;
            }
        } else if component.manager == ComponentManager::Generic {
            // Generic manager are typically applications
            component.component_type = ComponentType::Application;
        } else {
            // Default to library
            component.component_type = ComponentType::Library;
        }
    }

    /// Find or Create Component
    pub async fn find_or_create<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut select = Component::query_select()
            .where_eq("name", self.name.clone())
            .and()
            .where_eq("manager", self.manager.clone());

        if let Some(namespace) = &self.namespace {
            // HACK: Update this
            select = select.and().where_eq("namespace", namespace.clone());
        }

        let select_final = select.build()?;

        match Component::query_first(connection, select_final).await {
            Ok(dep) => {
                self.id = dep.id;
                Ok(())
            }
            Err(_) => self.save(connection).await.map_err(|e| e.into()),
        }
    }

    /// Get the top components
    pub async fn top<'a, T>(
        connection: &'a T,
        page: &Pagination,
    ) -> Result<Vec<Self>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Component::query(
            connection,
            Component::query_select()
                .where_ne("component_type", ComponentType::Library)
                .and()
                .where_ne("component_type", ComponentType::Unknown)
                .and()
                .where_ne("component_type", ComponentType::Framework)
                .page(page)
                .order_by("name", QueryOrder::Asc)
                .build()?,
        )
        .await?)
    }

    /// Find Component by Name
    pub async fn find_by_name<'a, T>(
        connection: &'a T,
        name: impl Into<String>,
        page: &Pagination,
    ) -> Result<Vec<Component>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let name = name.into();
        let select = Component::query_select()
            .where_like("name", format!("%{}%", name))
            .or()
            .where_like("namespace", format!("%{}%", name))
            .page(page)
            .build()?;

        Ok(Component::query(connection, select).await?)
    }

    /// Find Component by type
    pub async fn find_by_component_type<'a, T>(
        connection: &'a T,
        ctype: impl Into<ComponentType>,
        page: &Pagination,
    ) -> Result<Vec<Component>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let ctype = ctype.into();
        Ok(Self::query(
            connection,
            Self::query_select()
                .where_eq("component_type", ctype)
                .page(page)
                .build()?,
        )
        .await?)
    }
}

/// Component Dependency Model
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct ComponentVersion {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Component ID
    #[geekorm(foreign_key = "Component.id")]
    pub component_id: ForeignKey<i32, Component>,

    /// Version (semver or other format)
    pub version: String,
}

impl ComponentVersion {
    /// Semver Version
    pub fn version(&self) -> Result<semver::Version, crate::KonarrError> {
        Ok(semver::Version::parse(self.version.as_str())?)
    }

    /// Find or Create Component Version
    pub async fn find_or_crate<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let select = ComponentVersion::query_select()
            .where_eq("component_id", self.component_id.clone())
            .and()
            .where_eq("version", self.version.clone())
            .build()?;

        match ComponentVersion::query_first(connection, select).await {
            Ok(dep) => {
                self.id = dep.id;
                Ok(())
            }
            Err(_) => self.save(connection).await.map_err(|e| e.into()),
        }
    }
}

/// Dependency Manager Enum
///
/// https://github.com/package-url/purl-spec/blob/master/PURL-TYPES.rst
#[derive(Data, Debug, Default, Clone, PartialEq, Eq)]
#[geekorm(from_string = "lowercase", to_string = "lowercase")]
pub enum ComponentManager {
    /// Alpine Linux
    #[geekorm(aliases = "apk,alpine")]
    Apk,
    /// Cargo / Rust
    #[geekorm(aliases = "cargo,rust,rustc,rustlang")]
    Cargo,
    /// Composer / PHP
    #[geekorm(aliases = "composer,php")]
    Composer,
    /// Debian / Ubuntu
    #[geekorm(aliases = "deb,debian")]
    Deb,
    /// Ruby Gem
    #[geekorm(aliases = "gem,ruby")]
    Gem,
    /// Generic
    #[geekorm(aliases = "generic")]
    Generic,
    /// NPM
    #[geekorm(aliases = "npm,node,javascript")]
    Npm,
    /// Go Modules
    #[geekorm(aliases = "go,golang")]
    Golang,
    /// Maven / Java / Kotlin
    #[geekorm(aliases = "maven,gradle,java,kotlin,jvm")]
    Maven,
    /// Python Pip
    #[geekorm(aliases = "pypi,pip,python")]
    PyPi,
    /// Nuget
    #[geekorm(aliases = "nuget,csharp")]
    Nuget,
    /// RPM (Redhat Package Manager)
    #[geekorm(aliases = "rpm,redhat")]
    Rpm,
    /// Unknown Package Manager
    #[default]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing() {
        let debs = vec!["deb", "DeBiAn", "debian", "DEBIAN", "Debian"];
        for deb in debs.iter() {
            let pdeb = ComponentManager::from(*deb);
            assert_eq!(pdeb, ComponentManager::Deb);
            assert_eq!(pdeb.to_string(), "deb");
        }
    }

    #[test]
    fn test_purl_to_comp() {
        let purl = "pkg:apk/alpine".to_string();

        let (comp, version) = Component::from_purl(purl).unwrap();
        assert_eq!(comp.manager, ComponentManager::Apk);
        assert_eq!(comp.namespace, None);
        assert_eq!(comp.name, "alpine");
        assert_eq!(comp.component_type, ComponentType::OperatingSystem);

        // Default version is 0.0.0
        assert_eq!(version.version, "0.0.0".to_string());
    }

    #[test]
    fn test_purls() {
        let purls = vec!["pkg:deb/debian", "pkg:deb/debian/openssl", "pkg:apk/alpine"];

        for purl in purls.iter() {
            let (comp, _version) = Component::from_purl(purl.to_string()).unwrap();
            assert_eq!(comp.purl(), purl.to_string());
        }
    }
}
