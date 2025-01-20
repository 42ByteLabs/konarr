#![doc = r" GeekORM Database Migrations"]
#![allow(unused_imports, unused_variables)]
use geekorm::prelude::*;
mod v0_3_3;
pub use v0_3_3::{Database, Migration as LatestMigration};
#[doc = "Initializes and automatically migrates database."]
pub async fn init<'a, T>(connection: &'a T) -> Result<(), geekorm::Error>
where
    T: geekorm::GeekConnection<Connection = T> + 'a,
{
    let latest = &LatestMigration;
    match latest.validate_database(connection, &Database).await {
        Ok(MigrationState::Initialized) => {
            LatestMigration::create(connection).await?;
        }
        Ok(MigrationState::UpToDate) => {}
        Ok(MigrationState::OutOfDate(_)) => {
            return Err(geekorm::Error::Unknown);
        }
        Err(err) => {
            return Err(err);
        }
    }
    Ok(())
}
