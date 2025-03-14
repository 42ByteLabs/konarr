use konarr::{
    KONARR_BANNER, KONARR_VERSION, KonarrClient, KonarrError, client::projects::KonarrProjects,
};

#[tokio::main]
async fn main() -> Result<(), KonarrError> {
    println!("{}    v{}\n", KONARR_BANNER, KONARR_VERSION);

    println!("Creating Konarr Client");
    let token = std::env::var("KONARR_TOKEN").expect("KONARR_TOKEN is not set");
    let client = KonarrClient::init()
        .base("http://localhost:8000/api")?
        .token(token)
        .build()?;

    // Get the Server Information
    let server_info = client.server().await?;
    println!("Server Info: {:#?}", server_info);

    if !client.is_authenticated().await {
        println!("Client is not authenticated");
        return Err(KonarrError::AuthenticationError(
            "Client is not authenticated".to_string(),
        ));
    }

    // List Projects (paginated)
    let projects = KonarrProjects::list_top(&client).await?;
    println!("Total Projects: {}", projects.total);

    for project in projects.data {
        println!(
            "  > Project({}, '{}', {})",
            project.id, project.name, project.project_type
        );
        println!("    - {:?}", project.snapshot);
    }

    Ok(())
}
