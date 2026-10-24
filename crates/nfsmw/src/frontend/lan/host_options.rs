//! The [`HostOptions`] a host starts with, from the `net_*` settings and what the Host screen or `net.host` chose.

use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::{ServerName, Settings};
use nfsmw_net::settings::{Field, SettingsError, defaults, validate};

use super::backend::HostOptions;
use super::car::car_type;
use crate::i18n::{self, Args, t};
use crate::settings::net::NetSettings;

/// The room name when none is typed: `<player>'s game`, cut to the protocol's 32 characters.
pub fn default_room(player: &str) -> String {
    t!("lan-default-room", player = player).chars().take(32).collect()
}

/// The LAN defaults of `nfsmw_net::settings` for a room called `room`.
pub fn default_settings(room: &str) -> Result<Settings, String> {
    let name = ServerName::new(room).map_err(|_| t!("lan-room-name-bad"))?;
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
    let id = match error {
        SettingsError::Invalid(field) => return t!("lan-settings-invalid", field = field_name(*field)),
        SettingsError::Locked(field) => return t!("lan-settings-locked", field = field_name(*field)),
        SettingsError::BelowCount(field) => return t!("lan-settings-below-count", field = field_name(*field)),
        SettingsError::TooLarge => "lan-settings-too-large",
        SettingsError::NotPermitted => "lan-settings-not-permitted",
        SettingsError::NoPassword => "lan-settings-no-password",
        SettingsError::UnknownKey => "lan-settings-unknown-key",
    };
    i18n::text(id, &Args::new())
}

/// A settings field's name, as the Host form's rows and the refusals name it.
pub fn field_name(field: Field) -> String {
    let id = match field {
        Field::Name => "lan-field-name",
        Field::MaxPlayers => "lan-field-max-players",
        Field::MaxSpectators => "lan-field-max-spectators",
        Field::Mode => "lan-field-mode",
        Field::Track => "lan-field-track",
        Field::Laps => "lan-field-laps",
        Field::TrafficDensity => "lan-field-traffic-density",
        Field::CopLevelCap => "lan-field-cop-level-cap",
        Field::Pursuits => "lan-field-pursuits",
        Field::Collision => "lan-field-collision",
        Field::CatchUp => "lan-field-catch-up",
        Field::Nitrous => "lan-field-nitrous",
        Field::Damage => "lan-field-damage",
        Field::AllowedCars => "lan-field-allowed-cars",
        Field::JoinInProgress => "lan-field-join-in-progress",
        Field::PasswordRequired => "lan-field-password-required",
        Field::Public => "lan-field-public",
        Field::SimBuildMatch => "lan-field-sim-build-match",
    };
    i18n::text(id, &Args::new())
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
