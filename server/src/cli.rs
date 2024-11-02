use console::style;
use std::path::PathBuf;

use clap::Parser;
use konarr::{KONARR_BANNER, KONARR_VERSION};

pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Arguments {
    /// Enable debug mode
    #[clap(long, default_value_t = false)]
    pub debug: bool,

    /// Disable the banner
    #[clap(long, default_value_t = false)]
    pub disable_banner: bool,

    /// Path to the configuration file
    #[clap(short, long, default_value = "config/konarr.yml")]
    pub config: PathBuf,
}

pub fn init() -> Arguments {
    let arguments = Arguments::parse();

    // Setup logging and load .env file
    dotenvy::dotenv().ok();

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
            "{}    {} - v{}",
            style(KONARR_BANNER).green(),
            style(AUTHOR).red(),
            style(KONARR_VERSION).blue()
        );
    }

    arguments
}
