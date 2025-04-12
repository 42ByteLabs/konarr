use super::Migration;
use crate::models::Snapshot;

#[doc = "Migrations for 0.5.0"]
pub(super) async fn migrate<'a, C>(connection: &'a C) -> Result<(), geekorm::Error>
where
    C: geekorm::GeekConnection<Connection = C> + 'a,
{
    for snapshot in Snapshot::all(connection).await? {
        log::info!("Migrating snapshot: {}", snapshot.id);
    }

    Ok(())
}
