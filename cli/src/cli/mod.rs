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

    // Database Settings
    /// Database URL (SQLite)
    #[cfg(feature = "database")]
    #[clap(long, env = "KONARR_DB_URL")]
    pub database_url: Option<String>,

    // Agent Settings
    /// Monitoring Enabled
    #[clap(short, long, env = "KONARR_AGENT_MONITORING")]
    pub monitoring: bool,
    /// Agent Token
    #[clap(short, long, env = "KONARR_AGENT_TOKEN")]
    pub agent_token: Option<String>,
    /// Auto-Register Projects
    #[clap(long, env = "KONARR_AGENT_AUTO_CREATE")]
    pub auto_create: bool,
    /// Root Server Project ID
    #[clap(long, env = "KONARR_AGENT_PROJECT_ID")]
    pub project_id: Option<u32>,
    /// Agent Hostname
    #[clap(long, env = "KONARR_HOST")]
    pub hostname: Option<String>,

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
        #[clap(subcommand)]
        subcommands: Option<database::DatabaseCommands>,
    },
    #[cfg(feature = "database")]
    Display {
        #[clap(subcommand)]
        subcommands: Option<display::DisplayCommands>,
    },
    Agent {
        /// Docker Socket Path
        #[clap(short, long, env = "DOCKER_HOST")]
        docker_socket: Option<String>,
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
        .format_module_path(false)
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
