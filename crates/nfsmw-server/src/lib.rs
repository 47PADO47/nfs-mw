//! The multiplayer server loop (ADR 0011).
//!
//! - [`ServerConfig`]: the TOML config file and its checks (MP-25, MP-84).
//! - [`logging`]: the `log` backend (stderr and a file). [`timer`]: the drift-free tick timer.

pub mod config;
pub mod logging;
pub mod timer;

pub use config::{ConfigError, ServerConfig};
