//! The [`HostOptions`] a host starts with, from the `net_*` settings and what the Host screen or `net.host` chose.

use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{ServerName, Settings};
use nfsmw_net::settings::{SettingsError, defaults, validate};

use super::backend::HostOptions;
use super::car::car_type;
use crate::settings::net::NetSettings;

/// The room name when none is typed: `<player>'s game`, cut to the protocol's 32 characters.
pub fn default_room(player: &str) -> String {
    format!("{player}'s game").chars().take(32).collect()
}

/// The LAN defaults of `nfsmw_net::settings` for a room called `room`.
pub fn default_settings(room: &str) -> Result<Settings, String> {
    let name = ServerName::new(room).map_err(|e| format!("not a room name: {e:?}"))?;
    Ok(defaults(name))
}

/// The options of a host: `settings` checked, the password (empty is none) and `password_required` in step, the port
/// and the player's name from `net`, and the player's car.
pub fn host_options(net: &NetSettings, settings: Settings, password: &str, car: &str) -> Result<HostOptions, String> {
    let password = Some(password.trim()).filter(|p| !p.is_empty()).map(str::to_owned);
    let settings = Settings { password_required: password.is_some(), ..settings };
    validate(&settings).map_err(|e| settings_error(&e))?;
    Ok(HostOptions {
        settings,
        password,
        port: net.port,
        player_name: net.name.as_str().to_owned(),
        loadout: Loadout::stock(car_type(car)),
    })
}

/// Why settings were refused, as the screens and the console say it.
pub fn settings_error(error: &SettingsError) -> String {
    match error {
        SettingsError::Invalid(field) => format!("{field:?} is not valid with these settings"),
        SettingsError::TooLarge => "the settings are too large".into(),
        SettingsError::NotPermitted => "only the host or an admin may change the settings".into(),
        SettingsError::Locked(field) => format!("{field:?} cannot change during a race"),
        SettingsError::BelowCount(field) => format!("{field:?} is below the players already in"),
        SettingsError::NoPassword => "a password is required but none is set".into(),
        SettingsError::UnknownKey => "unknown setting".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsmw_net::message::Mode;

    #[test]
    fn the_room_takes_the_players_name_and_the_lan_defaults() {
        assert_eq!(default_room("Pietro"), "Pietro's game");
        assert_eq!(default_room(&"x".repeat(40)).chars().count(), 32);
        let settings = default_settings("Room").unwrap();
        assert_eq!((settings.max_players, settings.mode), (8, Mode::FreeRoam));
        assert!(default_settings("").is_err());
    }

    #[test]
    fn a_password_sets_the_flag_and_bad_settings_are_refused() {
        let net = NetSettings::default();
        let settings = default_settings("Room").unwrap();
        let open = host_options(&net, settings.clone(), "  ", "BMWM3GTR").unwrap();
        assert!(!open.settings.password_required && open.password.is_none());
        assert_eq!((open.port, open.player_name.as_str()), (9750, "Player"));
        assert_eq!(open.loadout.car_type, car_type("BMWM3GTR"));
        let locked = host_options(&net, settings.clone(), "hunter2", "BMWM3GTR").unwrap();
        assert!(locked.settings.password_required);
        assert_eq!(locked.password.as_deref(), Some("hunter2"));
        let circuit = Settings { mode: Mode::Circuit, laps: 0, ..settings };
        assert!(host_options(&net, circuit, "", "BMWM3GTR").unwrap_err().contains("Laps"));
    }
}
