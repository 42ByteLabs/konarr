use super::Migration;
#[doc = "Migrations for 0.5.0"]
pub(super) async fn migrate<'a, C>(connection: &'a C) -> Result<(), geekorm::Error>
where
    C: geekorm::GeekConnection<Connection = C> + 'a,
{
    let mut projects = crate::models::projects::Projects::fetch_all(connection).await?;
    for project in &mut projects {
        project.updated_at = chrono::Utc::now();
        project.save(connection).await?;
    }

    todo!("Migrate the database to version ")
}
