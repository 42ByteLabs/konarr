//! # Dependencies Model / Tables

use geekorm::{Connection, prelude::*};
use purl::GenericPurl;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub mod snapshots;

use super::{Component, ComponentManager, ComponentType, ComponentVersion};
use crate::bom::sbom::BomComponent;

pub use snapshots::Snapshot;

/// Dependency Model
#[derive(Table, Debug, Default, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    /// Primary Key
    #[geekorm(primary_key, auto_increment)]
    pub id: PrimaryKey<i32>,

    /// Snapshot ID
    #[geekorm(foreign_key = "Snapshot.id")]
    pub snapshot_id: ForeignKey<i32, Snapshot>,

    /// Dependency ID
    #[geekorm(foreign_key = "Component.id")]
    pub component_id: ForeignKey<i32, Component>,

    /// Dependency Version ID
    #[geekorm(foreign_key = "ComponentVersion.id")]
    pub component_version_id: ForeignKey<i32, ComponentVersion>,
}

impl Dependencies {
    /// Get component
    pub fn component(&self) -> Component {
        self.component_id.data.clone()
    }
    /// Get component ID
    pub fn component_id(&self) -> PrimaryKey<i32> {
        self.component_id.data.id
    }
    /// Get Component Type
    pub fn component_type(&self) -> ComponentType {
        self.component_id.data.component_type.clone()
    }
    /// Get manager
    pub fn manager(&self) -> ComponentManager {
        self.component_id.data.manager.clone()
    }
    /// Get name
    pub fn name(&self) -> String {
        self.component_id.data.name.clone()
    }
    /// Get namespace
    pub fn namespace(&self) -> Option<String> {
        self.component_id.data.namespace.clone()
    }
    /// Get version
    pub fn version(&self) -> Option<String> {
        Some(self.component_version_id.data.version.clone())
    }

    /// Package URL
    pub fn purl(&self) -> String {
        let mut purl = format!("pkg:{}", self.manager());
        if let Some(namespace) = &self.namespace() {
            purl.push_str(&format!("/{}/{}", namespace, self.name()));
        } else {
            purl.push_str(&format!("/{}", self.name()));
        }
        if let Some(version) = &self.version() {
            purl.push_str(&format!(":{}", version));
        }
        purl
    }

    /// Create a new Dependency from Package URL
    pub async fn from_purl<'a, T>(
        connection: &'a T,
        value: String,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let (mut component, mut version) = Component::from_purl(value)?;

        component.find_or_create(connection).await?;

        // Version needs to point to the component
        version.component_id = component.id.into();
        version.save(connection).await?;

        Ok(Dependencies::new(0, component.id, version.id))
    }

    /// Create new or find existing Dependency from BOM Component
    pub async fn from_bom_compontent<'a, T>(
        connection: &'a T,
        snapshop: impl Into<PrimaryKey<i32>>,
        bom_component: &BomComponent,
    ) -> Result<Self, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let snapshop = snapshop.into();
        let (mut component, mut version) = Component::from_purl(bom_component.purl.clone())?;
        // Update non-purl fields
        component.find_or_create(connection).await?;

        version.component_id = component.id.into();
        version.find_or_crate(connection).await?;

        // Remove duplicate dependencies
        match Dependencies::query_first(
            connection,
            Dependencies::query_select()
                .where_eq("snapshot_id", snapshop)
                .and()
                .where_eq("component_id", component.id)
                .and()
                .where_eq("component_version_id", version.id)
                .build()?,
        )
        .await
        {
            Ok(dep) => Ok(dep),
            Err(_) => {
                let mut new_dep = Dependencies::new(0, component.id, version.id);
                new_dep.snapshot_id = snapshop.into();
                new_dep.save(connection).await?;
                Ok(new_dep)
            }
        }
    }

    /// Find or Create a Dependency
    pub async fn find_or_crate<'a, T>(
        &mut self,
        connection: &'a T,
    ) -> Result<(), crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let mut select = Dependencies::query_select()
            .where_eq("name", self.name())
            .and()
            .where_eq("manager", self.manager());

        if let Some(namespace) = &self.namespace() {
            // HACK: Update this
            select = select.and().where_eq("namespace", namespace.clone());
        }

        let select_final = select.build()?;

        match Dependencies::query_first(connection, select_final).await {
            Ok(dep) => {
                self.id = dep.id;
                Ok(())
            }
            Err(_) => self.save(connection).await.map_err(|e| e.into()),
        }
    }

    /// Search for Dependencies
    pub async fn search<'a, T>(
        connection: &'a T,
        snapshot_id: impl Into<PrimaryKey<i32>>,
        search: impl Into<String>,
    ) -> Result<Vec<Dependencies>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let search = search.into();
        let snapshot_id = snapshot_id.into();

        let mut query = Component::query_select()
            .where_like("name", format!("%{}%", search))
            .or()
            .where_like("namespace", format!("%{}%", search))
            .or()
            .where_like("manager", format!("%{}%", search))
            .limit(10);

        let comps = Component::query(connection, query.build()?).await?;

        let mut deps = Vec::new();
        for comp in comps {
            let mut instances = Dependencies::query(
                connection,
                Dependencies::query_select()
                    .where_eq("snapshot_id", snapshot_id)
                    .where_eq("component_id", comp.id)
                    .build()?,
            )
            .await?;

            for inst in instances.iter_mut() {
                inst.fetch(connection).await?;
            }

            deps.append(&mut instances);
        }

        Ok(deps)
    }

    /// Find Dependencies by Name or Manager (Partial Match)
    pub async fn find_by_name<'a, T>(
        connection: &'a T,
        name: impl Into<String>,
    ) -> Result<Vec<Dependencies>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let name = name.into();

        let mut query = Component::query_select()
            .where_like("name", format!("%{}%", name))
            .or()
            .where_like("namespace", format!("%{}%", name))
            .or()
            .where_like("manager", format!("%{}%", name))
            .limit(10);

        let comps = Component::query(connection, query.build()?).await?;

        let mut deps = Vec::new();
        for comp in comps {
            let mut instances = Dependencies::fetch_by_component_id(connection, comp.id).await?;

            for inst in instances.iter_mut() {
                inst.fetch(connection).await?;
            }

            deps.append(&mut instances);
        }

        Ok(deps)
    }

    /// Find Dependencies by Package URL
    ///
    /// - Does not support version
    pub async fn find_by_purl<'a, T>(
        connection: &'a T,
        purl: impl Into<String>,
    ) -> Result<Vec<Dependencies>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        let purl = GenericPurl::<String>::from_str(purl.into().as_str())
            .map_err(|e| crate::KonarrError::UnknownError(e.to_string()))?;

        let manager = ComponentManager::from(purl.package_type());

        let mut query = Component::query_select()
            .where_eq("name", purl.name())
            .and()
            .where_eq("manager", manager);

        if let Some(namespace) = purl.namespace() {
            query = query.and().where_eq("namespace", namespace);
        }

        let comps = Component::query(connection, query.build()?).await?;

        let mut deps = Vec::new();
        for comp in comps {
            let mut instances = Dependencies::fetch_by_component_id(connection, comp.id).await?;

            for inst in instances.iter_mut() {
                inst.fetch(connection).await?;
            }

            deps.extend(instances);
        }

        Ok(deps)
    }

    /// Fetch Dependencies for a Snapshot
    pub async fn fetch_dependencies_by_snapshop<'a, T>(
        connection: &'a T,
        snapshop: impl Into<PrimaryKey<i32>>,
    ) -> Result<Vec<Dependencies>, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        // TODO: This could be a little more efficient and fetch all the content in one query
        let mut deps = Dependencies::query(
            connection,
            QueryBuilder::select()
                .table(Dependencies::table())
                .join(Component::table())
                .where_eq("snapshot_id", snapshop.into())
                .build()?,
        )
        .await?;

        for dep in deps.iter_mut() {
            dep.fetch(connection).await?;
        }

        Ok(deps)
    }

    /// Fetch single Dependency by snapshot ID
    pub async fn fetch_dependency_by_snapshot<'a, T>(
        connection: &'a T,
        snapshot: impl Into<PrimaryKey<i32>>,
        component: impl Into<PrimaryKey<i32>>,
    ) -> Result<Dependencies, crate::KonarrError>
    where
        T: GeekConnection<Connection = T> + 'a,
    {
        Ok(Dependencies::query_first(
            connection,
            Dependencies::query_select()
                .where_eq("snapshot_id", snapshot.into())
                .and()
                .where_eq("component_id", component.into())
                .build()?,
        )
        .await?)
    }

    /// Count all of the dependencies for a given Snapshot ID
    pub async fn count_by_snapshot(
        connection: &Connection<'_>,
        snapshot: impl Into<PrimaryKey<i32>>,
    ) -> Result<i64, crate::KonarrError> {
        Ok(Dependencies::row_count(
            connection,
            Dependencies::query_count()
                .where_eq("snapshot_id", snapshot.into())
                .build()?,
        )
        .await?)
    }
}
