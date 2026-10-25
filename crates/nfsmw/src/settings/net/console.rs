//! The LAN settings in the console: `get net_port`, `set net_port 9760`, `set net_show_stats` (a switch flips).

use super::super::partial::parse_bool;
use super::io::{
    IDENTITY_FILE_KEY, INTERP_DELAY_KEY, NAME_KEY, PORT_KEY, PUBLIC_ADDRESS_KEY, SHOW_STATS_KEY, SNAPSHOT_HZ_KEY,
};
use super::{
    IdentityPath, NetSettings, PublicAddress, parse_interp_delay, parse_port, parse_public_address, parse_snapshot_hz,
    valid_name,
};

/// Every console key of the LAN settings, in the order `get` lists them.
pub const KEYS: [&str; 7] =
    [NAME_KEY, PORT_KEY, SNAPSHOT_HZ_KEY, INTERP_DELAY_KEY, SHOW_STATS_KEY, IDENTITY_FILE_KEY, PUBLIC_ADDRESS_KEY];

/// Whether the console key names one of the LAN settings.
pub fn is_key(key: &str) -> bool {
    KEYS.contains(&key)
}

/// The switch a console key names, for `set <key>` with no value.
pub fn flag<'a>(settings: &'a mut NetSettings, key: &str) -> Option<&'a mut bool> {
    match key {
        SHOW_STATS_KEY => Some(&mut settings.show_stats),
        _ => None,
    }
}

/// The value as the console shows it.
pub fn get(settings: &NetSettings, key: &str) -> String {
    match key {
        NAME_KEY => settings.name.to_string(),
        PORT_KEY => settings.port.to_string(),
        SNAPSHOT_HZ_KEY => settings.snapshot_hz.to_string(),
        INTERP_DELAY_KEY => settings.interp_delay.to_string(),
        SHOW_STATS_KEY if settings.show_stats => "on".into(),
        SHOW_STATS_KEY => "off".into(),
        PUBLIC_ADDRESS_KEY if settings.public_address.is_empty() => "default".into(),
        PUBLIC_ADDRESS_KEY => settings.public_address.to_string(),
        _ if settings.identity_file.is_empty() => "default".into(),
        _ => settings.identity_file.to_string(),
    }
}

/// Sets a value from the console. `net_identity_file default` goes back to the default path.
pub fn set(settings: &mut NetSettings, key: &str, value: &str) -> Result<(), String> {
    match key {
        NAME_KEY => settings.name = valid_name(value)?,
        PORT_KEY => settings.port = parse_port(value)?,
        SNAPSHOT_HZ_KEY => settings.snapshot_hz = parse_snapshot_hz(value)?,
        INTERP_DELAY_KEY => settings.interp_delay = parse_interp_delay(value)?,
        SHOW_STATS_KEY => settings.show_stats = parse_bool(value)?,
        IDENTITY_FILE_KEY if value.trim() == "default" => settings.identity_file = IdentityPath::EMPTY,
        IDENTITY_FILE_KEY => settings.identity_file = IdentityPath::new(value)?,
        PUBLIC_ADDRESS_KEY if value.trim() == "default" => settings.public_address = PublicAddress::EMPTY,
        PUBLIC_ADDRESS_KEY => settings.public_address = parse_public_address(value)?,
        other => return Err(format!("unknown setting {other}")),
    }
    Ok(())
}

/// What each key accepts, for the console's usage line.
pub fn syntax(key: &str) -> Option<&'static str> {
    Some(match key {
        NAME_KEY => "<name, 1-24 characters>",
        PORT_KEY => "<1024-65534>",
        SNAPSHOT_HZ_KEY => "<1-60>",
        INTERP_DELAY_KEY => "<1-4 snapshot intervals>",
        SHOW_STATS_KEY => "<on|off>",
        IDENTITY_FILE_KEY => "<path|default>",
        PUBLIC_ADDRESS_KEY => "<ip|default>",
        _ => return None,
    })
}
