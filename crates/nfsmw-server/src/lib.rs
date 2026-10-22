//! The multiplayer server loop (ADR 0011).
//!
//! - [`ServerConfig`]: the TOML config file and its checks (MP-25, MP-84).

pub mod config;

pub use config::{ConfigError, ServerConfig};
