use clap::{Parser, Subcommand};
use console::style;
use konarr::{KONARR_BANNER, KONARR_VERSION};
use std::path::PathBuf;

pub mod agent;
#[cfg(feature = "database")]
pub mod database;
#[cfg(feature = "database")]
pub mod display;
#[cfg(feature = "database")]
pub mod index;
#[cfg(feature = "database")]
pub mod search;

pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Enable Debugging
    #[clap(long, env, default_value_t = false)]
    pub debug: bool,

    /// Disable Banner
    #[clap(long, default_value_t = false)]
    pub disable_banner: bool,

    /// Configuration file path
    #[clap(short, long, env, default_value = "./konarr.yml")]
    pub config: PathBuf,

    /// Working Directory
    #[clap(short, long, env, default_value = "./")]
    pub working_dir: PathBuf,

    #[clap(short, long, env = "KONARR_MONITORING")]
    pub monitoring: bool,

    /// If the command is running in a container
    #[clap(long, env = "KONARR_CONTAINER")]
    pub container: bool,

    /// Subcommands
    #[clap(subcommand)]
    pub commands: Option<ArgumentCommands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ArgumentCommands {
    #[cfg(feature = "database")]
    Database {
        #[clap(long, env = "DATABASE_URL")]
        database_url: Option<String>,

        #[clap(subcommand)]
        subcommands: Option<database::DatabaseCommands>,
    },
    #[cfg(feature = "database")]
    Display {
        #[clap(subcommand)]
        subcommands: Option<display::DisplayCommands>,
    },
    Agent {
        /// Root Server Project ID
        #[clap(long, env = "PROJECT_ID")]
        project_id: Option<u32>,

        #[clap(long, env = "HOST")]
        hostname: Option<String>,

        /// Agent Token
        #[clap(short, long, env = "KONARR_AGENT_TOKEN")]
        agent_token: Option<String>,

        /// Subcommands
        #[clap(subcommand)]
        subcommands: Option<agent::AgentCommands>,
    },
    #[cfg(feature = "database")]
    Index {
        #[clap(subcommand)]
        subcommands: Option<index::IndexCommand>,
    },
    #[cfg(feature = "database")]
    Search {
        #[clap(subcommand)]
        subcommands: Option<search::SearchCommands>,
    },
}

pub fn init() -> Arguments {
    let arguments = Arguments::parse();

    let log_level = match &arguments.debug {
        false => log::LevelFilter::Info,
        true => log::LevelFilter::Debug,
    };

    env_logger::builder()
        .parse_default_env()
        .filter_level(log_level)
        .init();

    if !arguments.disable_banner {
        println!(
            "{}    by {} - v{}\n",
            style(KONARR_BANNER).green(),
            style(AUTHOR).red(),
            style(KONARR_VERSION).blue()
        );
    }

    arguments
}
