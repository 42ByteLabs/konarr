use clap::{Parser, Subcommand};
use console::style;
use konarr::{Config, KONARR_BANNER, KONARR_VERSION};
use std::path::PathBuf;

pub mod agent;
#[cfg(feature = "database")]
pub mod database;
#[cfg(feature = "database")]
pub mod display;
#[cfg(feature = "database")]
pub mod generate;
#[cfg(feature = "database")]
pub mod index;
#[cfg(feature = "database")]
pub mod search;
#[cfg(feature = "tasks")]
pub mod tasks;

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

    /// Konarr Server URL
    #[clap(short, long, env = "KONARR_INSTANCE")]
    pub instance: Option<String>,

    // Agent Settings
    /// Monitoring Enabled
    #[clap(short, long, env = "KONARR_AGENT_MONITORING")]
    pub monitoring: bool,
    /// Agent Token
    #[clap(short, long, env = "KONARR_AGENT_TOKEN")]
    pub agent_token: Option<String>,
    /// Auto-Register Projects
    #[clap(long, env = "KONARR_AGENT_AUTO_CREATE", default_value = "false")]
    pub auto_create: bool,
    /// Root Server Project ID
    #[clap(long, env = "KONARR_AGENT_PROJECT_ID")]
    pub project_id: Option<u32>,
    /// Agent Hostname
    #[clap(long, env = "KONARR_AGENT_HOST")]
    pub hostname: Option<String>,

    /// Tool to use
    #[clap(short, long, env = "KONARR_AGENT_TOOL")]
    pub tool: Option<String>,
    /// Auto-Install Tools
    #[clap(long, env = "KONARR_AGENT_AUTO_INSTALL", default_value = "false")]
    pub auto_install: bool,
    #[clap(long, env = "KONARR_AGENT_AUTO_UPDATE", default_value = "false")]
    pub auto_update: bool,

    /// If the command is running in a container
    #[clap(long, env = "KONARR_CONTAINER")]
    pub container: bool,

    /// Subcommands
    #[clap(subcommand)]
    pub commands: Option<ArgumentCommands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ArgumentCommands {
    /// Database actions and commands
    #[cfg(feature = "database")]
    Database {
        #[clap(subcommand)]
        subcommands: Option<database::DatabaseCommands>,
    },
    /// Display data
    #[cfg(feature = "database")]
    Display {
        #[clap(subcommand)]
        subcommands: Option<display::DisplayCommands>,
    },
    /// Agent mode
    Agent {
        /// Docker Socket Path
        #[clap(short, long, env = "DOCKER_HOST")]
        docker_socket: Option<String>,
    },
    /// Scan a container image
    Scan {
        /// Image to scan
        #[clap(short, long)]
        image: Option<String>,
        /// List of tool
        #[clap(short, long)]
        list: bool,
        /// Output
        #[clap(short, long)]
        output: Option<String>,
    },
    /// Upload a SBOM file
    UploadSbom {
        /// Path to the file to upload
        #[clap(short, long)]
        input: PathBuf,
        /// Snapshot ID, if not provided a new snapshot will be created
        #[clap(short, long)]
        snapshot_id: Option<u32>,
    },
    /// Index data into the database
    #[cfg(feature = "database")]
    Index {
        #[clap(subcommand)]
        subcommands: Option<index::IndexCommand>,
    },
    /// Search the database for specific data
    #[cfg(feature = "database")]
    Search {
        #[clap(subcommand)]
        subcommands: Option<search::SearchCommands>,
    },

    /// Generate
    #[cfg(feature = "database")]
    Generate {
        #[clap(subcommand)]
        subcommands: Option<generate::GenerateCommands>,
    },

    /// Run various tasks
    #[cfg(feature = "tasks")]
    Tasks {
        #[clap(subcommand)]
        subcommands: Option<tasks::TaskCommands>,
    },
}

pub fn init() -> Arguments {
    dotenvy::dotenv().ok();
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

pub fn update_config(
    config: &mut Config,
    arguments: &Arguments,
) -> Result<(), konarr::KonarrError> {
    log::debug!("Updating configuration with arguments");
    if let Some(instance) = &arguments.instance {
        config.server.set_instance(&instance)?;
    }
    if let Some(token) = &arguments.agent_token {
        config.agent.token = Some(token.to_string());
    }
    config.agent.project_id = arguments.project_id;
    config.agent.create = arguments.auto_create;
    if let Some(hostname) = &arguments.hostname {
        config.agent.host = Some(hostname.to_string());
    }
    // Tool settings
    if let Some(tool) = &arguments.tool {
        config.agent.tool = Some(tool.to_string());
    }
    config.agent.tool_auto_install = arguments.auto_install;
    config.agent.tool_auto_update = arguments.auto_update;
    Ok(())
}
