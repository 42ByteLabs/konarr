use anyhow::Result;
use clap::Subcommand;
use geekorm::prelude::*;
use log::{debug, info};

use konarr::{Config, models::UserRole};

#[derive(Subcommand, Debug, Clone)]
pub enum DatabaseCommands {
    Create {},
    /// Create a new user
    #[clap(visible_alias = "create-user")]
    User {},
}

pub async fn run(config: &mut Config, subcommands: Option<DatabaseCommands>) -> Result<()> {
    println!("Config :: {:#?}", config.database);
    let db = config.database().await?;

    info!("Connected!");

    match subcommands {
        Some(DatabaseCommands::Create {}) => {
            konarr::models::database_initialise(config).await?;
        }
        Some(DatabaseCommands::User {}) => {
            let username = crate::utils::interactive::prompt_input("Username")?;
            let password = crate::utils::interactive::prompt_password("Password")?;
            let role_str = crate::utils::interactive::prompt_select_with_default(
                "Role",
                &vec!["Admin", "User"],
                0,
            )?;
            let role = UserRole::from(role_str.0);

            let mut session = konarr::models::Sessions::new(
                konarr::models::SessionType::User,
                konarr::models::SessionState::Inactive,
            );
            session.save(&db.acquire().await).await?;

            let mut new_user = konarr::models::Users::new(username, password, role, session.id);
            new_user.save(&db.acquire().await).await?;

            info!("User created successfully");
        }
        None => {
            debug!("No subcommand provided, running interactive mode");

            let (action, id) = crate::utils::interactive::prompt_select(
                "Database Action",
                &vec!["Create Database"],
            )?;

            debug!("Selected Action: {}", action);

            match id {
                0 => {
                    konarr::models::database_initialise(config).await?;
                }
                _ => {
                    info!("No action selected");
                }
            }
        }
    }
    Ok(())
}
