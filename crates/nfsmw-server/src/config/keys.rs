//! One key of the config file applied to a [`ServerConfig`]: the server's own keys here, the session keys through
//! [`nfsmw_net::settings::set`].

use std::net::IpAddr;
use std::path::PathBuf;

use log::LevelFilter;
use nfsmw_net::settings::{Field, set};
use toml::Value;

use super::{ConfigError, MAX_SNAPSHOT_BUDGET, ServerConfig};

/// Applies `key = value` to `config`.
pub(super) fn apply(config: &mut ServerConfig, key: &str, value: &Value) -> Result<(), ConfigError> {
    if Field::from_key(key).is_some() {
        let text = scalar_text(key, value)?;
        return set(&mut config.settings, key, &text)
            .map_err(|error| ConfigError::Setting { key: key.to_owned(), error });
    }
    match key {
        "bind" => config.bind = string(key, value)?.parse::<IpAddr>().map_err(|_| typed(key, "an IP address"))?,
        "port" => config.port = int_in(key, value, 0, i64::from(u16::MAX))? as u16,
        "discovery" => config.discovery = boolean(key, value)?,
        "password" => return password(key, value),
        "snapshot_hz" => config.snapshot_hz = int_in(key, value, 1, 60)? as u8,
        "snapshot_budget" => {
            config.snapshot_budget = int_in(key, value, 1024, i64::from(MAX_SNAPSHOT_BUDGET))? as u32;
        }
        "relevancy_radius" => config.relevancy_radius = radius(key, value)?,
        "auto_start" => config.auto_start = boolean(key, value)?,
        "countdown_seconds" => config.countdown_seconds = int_in(key, value, 1, 30)? as u8,
        "results_seconds" => config.results_seconds = int_in(key, value, 1, 600)? as u16,
        "replay_dir" => config.replay_dir = optional_path(key, value)?,
        "log_file" => config.log_file = optional_path(key, value)?,
        "log_level" => config.log_level = level(key, value)?,
        "status_seconds" => config.status_seconds = int_in(key, value, 0, 3600)? as u32,
        _ => return Err(ConfigError::UnknownKey(key.to_owned())),
    }
    Ok(())
}

/// A session value as the text `settings::set` parses.
fn scalar_text(key: &str, value: &Value) -> Result<String, ConfigError> {
    match value {
        Value::String(text) => Ok(text.clone()),
        Value::Integer(number) => Ok(number.to_string()),
        Value::Boolean(flag) => Ok(flag.to_string()),
        _ => Err(typed(key, "a string, an integer or a bool")),
    }
}

/// A non-empty password is refused until the PAKE exchange runs in the server; an empty one means none.
fn password(key: &str, value: &Value) -> Result<(), ConfigError> {
    if string(key, value)?.is_empty() {
        return Ok(());
    }
    Err(ConfigError::Unsupported {
        key: key.to_owned(),
        reason: "the room password needs the PAKE exchange, which the server does not run yet; leave it empty",
    })
}

fn string<'a>(key: &str, value: &'a Value) -> Result<&'a str, ConfigError> {
    value.as_str().ok_or_else(|| typed(key, "a string"))
}

fn boolean(key: &str, value: &Value) -> Result<bool, ConfigError> {
    value.as_bool().ok_or_else(|| typed(key, "true or false"))
}

fn int_in(key: &str, value: &Value, min: i64, max: i64) -> Result<i64, ConfigError> {
    let number = value.as_integer().ok_or_else(|| typed(key, "an integer"))?;
    if number < min || number > max {
        return Err(ConfigError::Range { key: key.to_owned(), reason: range_text(min, max) });
    }
    Ok(number)
}

fn radius(key: &str, value: &Value) -> Result<f32, ConfigError> {
    let number = match value {
        Value::Integer(number) => *number as f64,
        Value::Float(number) => *number,
        _ => return Err(typed(key, "a number of metres")),
    };
    if !number.is_finite() || !(10.0..=20_000.0).contains(&number) {
        return Err(ConfigError::Range { key: key.to_owned(), reason: "between 10 and 20000 metres" });
    }
    Ok(number as f32)
}

/// A path, or none for an empty string.
fn optional_path(key: &str, value: &Value) -> Result<Option<PathBuf>, ConfigError> {
    let text = string(key, value)?;
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(text)))
}

fn level(key: &str, value: &Value) -> Result<LevelFilter, ConfigError> {
    let text = string(key, value)?;
    text.parse::<LevelFilter>().map_err(|_| typed(key, "one of off, error, warn, info, debug, trace"))
}

fn typed(key: &str, expected: &'static str) -> ConfigError {
    ConfigError::Type { key: key.to_owned(), expected }
}

/// The range texts the integer keys use.
fn range_text(min: i64, max: i64) -> &'static str {
    match (min, max) {
        (0, 65535) => "a port between 0 and 65535",
        (1, 60) => "between 1 and 60",
        (1024, _) => "between 1024 and 32768 bytes per second",
        (1, 30) => "between 1 and 30 seconds",
        (1, 600) => "between 1 and 600 seconds",
        (0, 3600) => "between 0 and 3600 seconds",
        _ => "outside the accepted range",
    }
}
