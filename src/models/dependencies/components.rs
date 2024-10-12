//! # Dependency Components Models / Tables

use std::{collections::HashMap, fmt::Display, str::FromStr};

use geekorm::prelude::*;
use log::debug;
use purl::GenericPurl;
use serde::{Deserialize, Serialize};

use super::ComponentType;

const COMPONENTS: &str = include_str!("./comps.yml");

type CompYml = HashMap<String, Vec<String>>;

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
    /// Initialise Component
    pub async fn init<'a, T>(connection: &'a T) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        debug!("Creating and Initialising Component Table");
        Component::create_table(connection).await?;

        let comps: CompYml = serde_yaml::from_str(COMPONENTS)?;
        debug!("Creating Standard Components: {}", comps.len());
        for (name, purls) in &comps {
            let comp_type = ComponentType::from(name);
            for purl in purls {
                let (mut component, _) = Component::from_purl(purl)?;
                component.component_type = comp_type.clone();
                component.fetch_or_create(connection).await?;
            }
        }

        Ok(())
    }

    /// Create PURL from Component
    pub fn purl(&self) -> String {
        let mut purl = format!("pkg:{}/", self.manager);

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

    /// Find or Create Component
    pub async fn find_or_crate<'a, T>(
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

    /// Find Component by Name
    pub async fn find_by_name<'a, T>(
        connection: &'a T,
        name: impl Into<String>,
        page: usize,
        limit: usize,
    ) -> Result<Vec<Component>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let name = name.into();
        let select = Component::query_select()
            .where_like("name", format!("%{}%", name))
            .or()
            .where_like("namespace", format!("%{}%", name))
            .limit(limit)
            .offset(page * limit)
            .build()?;

        Ok(Component::query(connection, select).await?)
    }

    /// Find Component by type
    pub async fn find_by_component_type<'a, T>(
        connection: &'a T,
        ctype: impl Into<ComponentType>,
        page: usize,
        limit: usize,
    ) -> Result<Vec<Component>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let ctype = ctype.into();
        Ok(Self::query(
            connection,
            Self::query_select()
                .where_eq("component_type", ctype)
                .limit(limit)
                .offset(page * limit)
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

    /// Version
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
#[derive(Data, Debug, Default, Clone)]
pub enum ComponentManager {
    /// Alpine Linux
    Apk,
    /// Cargo / Rust
    Cargo,
    /// Composer / PHP
    Composer,
    /// Debian / Ubuntu
    Deb,
    /// Ruby Gem
    Gem,
    /// Generic
    Generic,
    /// NPM
    Npm,
    /// Go Modules
    Golang,
    /// Maven / Java / Kotlin
    Maven,
    /// Python Pip
    PyPi,
    /// Nuget
    Nuget,
    /// RPM (Redhat Package Manager)
    Rpm,
    /// Unknown Package Manager
    #[default]
    Unknown,
}

impl Display for ComponentManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentManager::Apk => write!(f, "apk"),
            ComponentManager::Composer => write!(f, "composer"),
            ComponentManager::Cargo => write!(f, "cargo"),
            ComponentManager::Deb => write!(f, "deb"),
            ComponentManager::Gem => write!(f, "gem"),
            ComponentManager::Golang => write!(f, "golang"),
            ComponentManager::Generic => write!(f, "generic"),
            ComponentManager::Maven => write!(f, "maven"),
            ComponentManager::Npm => write!(f, "npm"),
            ComponentManager::Nuget => write!(f, "nuget"),
            ComponentManager::PyPi => write!(f, "pypi"),
            ComponentManager::Rpm => write!(f, "rpm"),
            ComponentManager::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&String> for ComponentManager {
    fn from(value: &String) -> Self {
        match value.to_lowercase().as_str() {
            "apk" | "alpine" => ComponentManager::Apk,
            "cargo" | "rust" => ComponentManager::Cargo,
            "composer" | "php" => ComponentManager::Composer,
            "deb" | "debian" => ComponentManager::Deb,
            "gem" | "ruby" => ComponentManager::Gem,
            "go" | "golang" => ComponentManager::Golang,
            "generic" => ComponentManager::Generic,
            "maven" | "java" | "kotlin" => ComponentManager::Maven,
            "npm" | "node" | "javascript" => ComponentManager::Npm,
            "pypi" | "pip" | "python" => ComponentManager::PyPi,
            "nuget" | "csharp" => ComponentManager::Nuget,
            "rpm" | "redhat" => ComponentManager::Rpm,
            _ => {
                log::warn!("Unknown Package Manager: {}", value);
                ComponentManager::Unknown
            }
        }
    }
}
