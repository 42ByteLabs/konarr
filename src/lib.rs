//! # Konarr
//!
//! Konarr is a monitoring tool for Containers, Kubernetes, and other
//!
//! Secure your Homelabs and Production environments with Konarr.
//!
#![deny(missing_docs, unused_imports)]
#![allow(clippy::result_large_err)]
#![deny(unsafe_code)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "sbom")]
pub mod bom;
#[cfg(feature = "client")]
pub mod client;
pub mod error;
#[cfg(feature = "tasks")]
pub mod tasks;
#[cfg(feature = "tools")]
pub mod tools;
pub mod utils;

#[cfg(feature = "models")]
pub mod db;
#[cfg(feature = "models")]
pub mod models;

pub use error::KonarrError;
pub use utils::config::Config;

#[cfg(feature = "client")]
pub use client::KonarrClient;

/// Konarr Banner
pub const KONARR_BANNER: &str = r#" _   __
| | / /
| |/ /  ___  _ __   __ _ _ __ _ __
|    \ / _ \| '_ \ / _` | '__| '__|
| |\  \ (_) | | | | (_| | |  | |
\_| \_/\___/|_| |_|\__,_|_|  |_|"#;

/// Konarr Version
pub const KONARR_VERSION: &str = env!("CARGO_PKG_VERSION");
