use konarr::{
    client::projects::KonarrProjects, KonarrClient, KonarrError, KONARR_BANNER, KONARR_VERSION,
};

#[tokio::main]
async fn main() -> Result<(), KonarrError> {
    println!("{}    v{}", KONARR_BANNER, KONARR_VERSION);
    println!("Creating Konarr Client");

    let token = std::env::var("KONARR_TOKEN").expect("KONARR_TOKEN is not set");
    let client = KonarrClient::init()
        .base("http://localhost:8080")?
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
    let projects = KonarrProjects::list(&client).await?;
    println!("Total Projects: {}", projects.total);

    for project in projects.data {
        println!(
            "  > Project({}, {}, {})",
            project.id, project.name, project.r#type
        );
    }

    Ok(())
}
