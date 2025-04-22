use super::Migration;
#[doc = "Migrations for 0.5.0"]
pub(super) async fn migrate<'a, C>(connection: &'a C) -> Result<(), geekorm::Error>
where
    C: geekorm::GeekConnection<Connection = C> + 'a,
{
    todo!("Migrate the database to version ")
}
