use anyhow::Result;
use clap::Subcommand;
use geekorm::prelude::*;
use konarr::models::Users;
use konarr::tasks::TaskTrait;
use konarr::tasks::cleanup::CleanupTask;
use log::{debug, info};

use konarr::{Config, models::UserRole};

#[derive(Subcommand, Debug, Clone)]
pub enum DatabaseCommands {
    Create {},
    /// Create a new user
    #[clap(visible_alias = "create-user")]
    User {},
    /// Cleanup the database
    Cleanup {
        #[clap(long, short)]
        force: bool,
    },
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

            // Find or create the user
            let connection = db.acquire().await;

            match Users::fetch_by_username(&connection, &username).await {
                Ok(mut user) => {
                    info!("User already exists");
                    info!("Updating user password and role");
                    user.hash_password(password)?;
                    user.role = role;
                    info!("Saving user");
                    user.update(&connection).await?;
                    info!("User updated: {:?}", user);
                }
                Err(err) => {
                    debug!("User not found: {}", err);
                    info!("Creating new user");
                    let mut user = Users::new(&username, &password, role.clone(), session.id);
                    user.save(&connection).await?;
                    info!("User created: {:?}", user);
                }
            };

            info!("User created successfully");
        }
        Some(DatabaseCommands::Cleanup { force }) => {
            let task = if force {
                CleanupTask::force()
            } else {
                CleanupTask::default()
            };
            task.run(&db).await?;
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
