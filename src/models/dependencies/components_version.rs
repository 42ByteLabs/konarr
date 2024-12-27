//! # Component Version Model

use geekorm::prelude::*;
use serde::{Deserialize, Serialize};

use super::components::Component;

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

    purl: Option<String>,

    cpe: Option<String>,
}

impl ComponentVersion {
    /// Semver Version
    pub fn version(&self) -> Result<semver::Version, crate::KonarrError> {
        Ok(semver::Version::parse(self.version.as_str())?)
    }

    /// Get the purl for the component version
    pub fn purl(&self) -> String {
        if let Some(purl) = self.purl.clone() {
            purl
        } else {
            self.component_id.data.purl()
        }
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
