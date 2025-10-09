//! # Dependencies Task
use std::collections::HashSet;

use crate::models::{Component, ComponentVersion, Dependencies};

use super::TaskTrait;
use geekorm::{ConnectionManager, GeekConnector};

/// DependenciesTask is for managing and deduplicating dependencies in the database
#[derive(Default)]
pub struct DependenciesTask;

#[async_trait::async_trait]
impl TaskTrait for DependenciesTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        log::info!("Starting Dependencies Task");
        let mut dependencies = Component::all(&database.acquire().await).await?;

        deduplicate_dependencies(database, &mut dependencies).await?;

        // Your task implementation here
        Ok(())
    }
}

async fn deduplicate_dependencies(
    database: &ConnectionManager,
    components: &mut Vec<Component>,
) -> Result<(), crate::KonarrError> {
    // Name of the components that are duplicates
    let mut unique_components: HashSet<String> = HashSet::new();

    for component in components.iter() {
        let purl = component.purl();
        log::debug!("Processing component: {}", purl);

        if unique_components.contains(&purl) {
            log::warn!("Duplicate component found: {}", component.name);
            // This is a duplicate component
            // Move or merge data from the duplicate component to the unique one

            let original = components.iter().find(|c| c.purl() == purl).unwrap();
            log::info!("Found original component: {:?}", original);

            let dependencies =
                Dependencies::fetch_by_component_id(&database.acquire().await, component.id)
                    .await?;

            // If there are no dependencies, we can safely delete the duplicate component
            if dependencies.is_empty() {
                log::warn!(
                    "No dependencies found for duplicate component ID: {}",
                    component.id
                );
                log::info!("Deleting duplicate component ID: {}", component.id);

                // Delete the version first due to foreign key constraints
                let versions = ComponentVersion::fetch_by_component_id(
                    &database.acquire().await,
                    component.id,
                )
                .await?;

                for ver in versions {
                    log::info!("Deleting component version ID: {}", ver.id);
                    ver.delete(&database.acquire().await).await?;
                }

                component.delete(&database.acquire().await).await?;
                continue;
            }

            if original.id == component.id {
                log::warn!(
                    "Original and duplicate component IDs are the same: {}",
                    original.id
                );
            }
            log::info!("Duplicate component: {:?}", component);
            log::info!("Found dependency: {}", dependencies.len());

            // Migrate dependencies to the original component
            for mut dep in dependencies {
                log::info!(
                    "Migrating dependency ID: {} from component ID: {} to original component ID: {}",
                    dep.id,
                    component.id,
                    original.id
                );

                dep.component_id = original.id.into();
                dep.update(&database.acquire().await).await?;
                log::info!("Migrated dependency ID: {}", dep.id);
                //
            }

            continue;
        }

        unique_components.insert(purl);
    }

    Ok(())
}
