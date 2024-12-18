//! # Task - Catalogue
use geekorm::prelude::*;

use crate::{
    models::{Component, ComponentType},
    utils::catalogue::Catalogue,
};

/// Catalogue the components task
pub async fn catalogue<'a, T>(connection: &'a T, force: bool) -> Result<(), crate::KonarrError>
where
    T: GeekConnection<Connection = T> + 'a,
{
    log::info!("Starting Catalogue Task");
    let catalogue = Catalogue::new();

    let mut counter = 0;
    let mut comps = Component::fetch_all(connection).await?;
    log::debug!("Checking component types for `{}` Components", comps.len());

    for mut comp in comps.iter_mut() {
        if !force {
            match comp.component_type {
                ComponentType::Unknown | ComponentType::Library | ComponentType::Application => {
                    if catalogue.catalogue(&mut comp)? {
                        log::info!("Updating component_type: {}", comp.component_type);
                        comp.update(connection).await?;
                        counter += 1;
                    }
                }
                _ => {}
            }
        } else {
            if catalogue.catalogue(&mut comp)? {
                log::info!("Updating component_type: {}", comp.component_type);
                comp.update(connection).await?;
                counter += 1;
            }
        }
    }
    if counter != 0 {
        log::info!("Updated `{}` component out of `{}`", counter, comps.len());
    }

    Ok(())
}
