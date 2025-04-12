//! # Task - Catalogue
use geekorm::{ConnectionManager, prelude::*};

use super::TaskTrait;
use crate::{
    models::{Component, ComponentType},
    utils::catalogue::Catalogue,
};

/// Catalogue the components task
#[derive(Default)]
pub struct CatalogueTask {
    force: bool,
}

#[async_trait::async_trait]
impl TaskTrait for CatalogueTask {
    async fn run(&self, database: &ConnectionManager) -> Result<(), crate::KonarrError> {
        log::info!("Starting Catalogue Task");
        let catalogue = Catalogue::new();

        let mut counter = 0;
        let mut comps = Component::all(&database.acquire().await).await?;
        log::debug!("Checking component types for `{}` Components", comps.len());

        for mut comp in comps.iter_mut() {
            if !self.force {
                match comp.component_type {
                    ComponentType::Unknown
                    | ComponentType::Library
                    | ComponentType::Application => {
                        if catalogue.catalogue(&mut comp)? {
                            log::info!("Updating component_type: {}", comp.component_type);
                            comp.update(&database.acquire().await).await?;
                            counter += 1;
                        }
                    }
                    _ => {}
                }
            } else if self.force && catalogue.catalogue(&mut comp)? {
                log::info!("Updating component_type: {}", comp.component_type);
                comp.update(&database.acquire().await).await?;
                counter += 1;
            }
        }
        if counter != 0 {
            log::info!("Updated `{}` component out of `{}`", counter, comps.len());
        }

        Ok(())
    }
}
