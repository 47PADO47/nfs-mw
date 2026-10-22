//! The flat key and value form of the settings: stable lower-case keys and text values, for a config file.
//!
//! Values: numbers in decimal, `true` or `false`, and these words: mode `free_roam`, `circuit`, `sprint`, `drag`,
//! `speedtrap`, `knockout`, `pursuit_coop`, `cops_vs_racers`, `tag` or `custom:<128..=255>`; collision `full`, `ghost`
//! or `ghost_start`; damage `visual` or `off`; join policy `closed`, `free_roam` or `between_races`; allowed cars `any`,
//! `cars:<id>,<id>,...`, `class:<n>` or `cap:<n>`.

use super::rules::{SettingsError, validate};
use crate::loadout::CarRestriction;
use crate::message::{CollisionMode, DamageMode, JoinInProgress, List, Mode, ServerName, Settings};

/// One editable settings field, by its stable key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Field {
    Name,
    MaxPlayers,
    MaxSpectators,
    Mode,
    Track,
    Laps,
    TrafficDensity,
    CopLevelCap,
    Pursuits,
    Collision,
    CatchUp,
    Nitrous,
    Damage,
    AllowedCars,
    JoinInProgress,
    PasswordRequired,
    Public,
    SimBuildMatch,
}

impl Field {
    /// Every field, in the struct's order.
    pub const ALL: [Field; 18] = [
        Self::Name,
        Self::MaxPlayers,
        Self::MaxSpectators,
        Self::Mode,
        Self::Track,
        Self::Laps,
        Self::TrafficDensity,
        Self::CopLevelCap,
        Self::Pursuits,
        Self::Collision,
        Self::CatchUp,
        Self::Nitrous,
        Self::Damage,
        Self::AllowedCars,
        Self::JoinInProgress,
        Self::PasswordRequired,
        Self::Public,
        Self::SimBuildMatch,
    ];

    /// The stable key.
    pub fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::MaxPlayers => "max_players",
            Self::MaxSpectators => "max_spectators",
            Self::Mode => "mode",
            Self::Track => "track",
            Self::Laps => "laps",
            Self::TrafficDensity => "traffic_density",
            Self::CopLevelCap => "cop_level_cap",
            Self::Pursuits => "pursuits",
            Self::Collision => "collision",
            Self::CatchUp => "catch_up",
            Self::Nitrous => "nitrous",
            Self::Damage => "damage",
            Self::AllowedCars => "allowed_cars",
            Self::JoinInProgress => "join_in_progress",
            Self::PasswordRequired => "password_required",
            Self::Public => "public",
            Self::SimBuildMatch => "sim_build_match",
        }
    }

    /// The field with key `key`, or `None`.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|field| field.key() == key)
    }

    /// Whether the field may change while a race loads or runs: the name, the spectator limit, the join policy, the
    /// password flag and the public flag. Everything else changes the race.
    pub fn live_editable(self) -> bool {
        matches!(self, Self::Name | Self::MaxSpectators | Self::JoinInProgress | Self::PasswordRequired | Self::Public)
    }
}

/// Every field of `settings` as a key and its text value, in [`Field::ALL`] order.
pub fn to_pairs(settings: &Settings) -> Vec<(&'static str, String)> {
    Field::ALL.into_iter().map(|field| (field.key(), value(field, settings))).collect()
}

/// `defaults` with each pair set in order, then [`validate`]d. An unknown key is [`SettingsError::UnknownKey`]; a
/// value that does not parse is [`SettingsError::Invalid`].
pub fn from_pairs<'a>(
    defaults: Settings,
    pairs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<Settings, SettingsError> {
    let mut settings = defaults;
    for (key, text) in pairs {
        set(&mut settings, key, text)?;
    }
    validate(&settings)?;
    Ok(settings)
}

/// Sets the field with key `key` from `text`. Only the parse is checked here; call [`validate`] on the result.
pub fn set(settings: &mut Settings, key: &str, text: &str) -> Result<(), SettingsError> {
    let field = Field::from_key(key).ok_or(SettingsError::UnknownKey)?;
    let bad = SettingsError::Invalid(field);
    let text = text.trim();
    match field {
        Field::Name => settings.name = ServerName::new(text).map_err(|_| bad)?,
        Field::MaxPlayers => settings.max_players = text.parse().map_err(|_| bad)?,
        Field::MaxSpectators => settings.max_spectators = text.parse().map_err(|_| bad)?,
        Field::Mode => settings.mode = parse_mode(text).ok_or(bad)?,
        Field::Track => settings.track = text.parse().map_err(|_| bad)?,
        Field::Laps => settings.laps = text.parse().map_err(|_| bad)?,
        Field::TrafficDensity => settings.traffic_density = text.parse().map_err(|_| bad)?,
        Field::CopLevelCap => settings.cop_level_cap = text.parse().map_err(|_| bad)?,
        Field::Pursuits => settings.pursuits = text.parse().map_err(|_| bad)?,
        Field::Collision => settings.collision = parse_collision(text).ok_or(bad)?,
        Field::CatchUp => settings.catch_up = text.parse().map_err(|_| bad)?,
        Field::Nitrous => settings.nitrous = text.parse().map_err(|_| bad)?,
        Field::Damage => settings.damage = parse_damage(text).ok_or(bad)?,
        Field::AllowedCars => settings.allowed_cars = parse_cars(text).ok_or(bad)?,
        Field::JoinInProgress => settings.join_in_progress = parse_join(text).ok_or(bad)?,
        Field::PasswordRequired => settings.password_required = text.parse().map_err(|_| bad)?,
        Field::Public => settings.public = text.parse().map_err(|_| bad)?,
        Field::SimBuildMatch => settings.sim_build_match = text.parse().map_err(|_| bad)?,
    }
    Ok(())
}

/// The text value of `field` in `settings`.
pub(crate) fn value(field: Field, settings: &Settings) -> String {
    match field {
        Field::Name => settings.name.to_string(),
        Field::MaxPlayers => settings.max_players.to_string(),
        Field::MaxSpectators => settings.max_spectators.to_string(),
        Field::Mode => mode_text(settings.mode),
        Field::Track => settings.track.to_string(),
        Field::Laps => settings.laps.to_string(),
        Field::TrafficDensity => settings.traffic_density.to_string(),
        Field::CopLevelCap => settings.cop_level_cap.to_string(),
        Field::Pursuits => settings.pursuits.to_string(),
        Field::Collision => collision_text(settings.collision).to_owned(),
        Field::CatchUp => settings.catch_up.to_string(),
        Field::Nitrous => settings.nitrous.to_string(),
        Field::Damage => damage_text(settings.damage).to_owned(),
        Field::AllowedCars => cars_text(&settings.allowed_cars),
        Field::JoinInProgress => join_text(settings.join_in_progress).to_owned(),
        Field::PasswordRequired => settings.password_required.to_string(),
        Field::Public => settings.public.to_string(),
        Field::SimBuildMatch => settings.sim_build_match.to_string(),
    }
}

const MODES: [(Mode, &str); 9] = [
    (Mode::FreeRoam, "free_roam"),
    (Mode::Circuit, "circuit"),
    (Mode::Sprint, "sprint"),
    (Mode::Drag, "drag"),
    (Mode::Speedtrap, "speedtrap"),
    (Mode::Knockout, "knockout"),
    (Mode::PursuitCoop, "pursuit_coop"),
    (Mode::CopsVsRacers, "cops_vs_racers"),
    (Mode::Tag, "tag"),
];

fn mode_text(mode: Mode) -> String {
    if let Mode::Custom(id) = mode {
        return format!("custom:{id}");
    }
    MODES.iter().find(|(known, _)| *known == mode).map_or_else(String::new, |(_, name)| (*name).to_owned())
}

fn parse_mode(text: &str) -> Option<Mode> {
    if let Some(id) = text.strip_prefix("custom:") {
        let id: u8 = id.parse().ok()?;
        return (id >= 128).then_some(Mode::Custom(id));
    }
    MODES.iter().find(|(_, name)| *name == text).map(|(mode, _)| *mode)
}

fn collision_text(mode: CollisionMode) -> &'static str {
    match mode {
        CollisionMode::Full => "full",
        CollisionMode::Ghost => "ghost",
        CollisionMode::GhostStart => "ghost_start",
    }
}

fn parse_collision(text: &str) -> Option<CollisionMode> {
    [CollisionMode::Full, CollisionMode::Ghost, CollisionMode::GhostStart]
        .into_iter()
        .find(|mode| collision_text(*mode) == text)
}

fn damage_text(mode: DamageMode) -> &'static str {
    match mode {
        DamageMode::Visual => "visual",
        DamageMode::Off => "off",
    }
}

fn parse_damage(text: &str) -> Option<DamageMode> {
    [DamageMode::Visual, DamageMode::Off].into_iter().find(|mode| damage_text(*mode) == text)
}

fn join_text(policy: JoinInProgress) -> &'static str {
    match policy {
        JoinInProgress::Closed => "closed",
        JoinInProgress::FreeRoam => "free_roam",
        JoinInProgress::BetweenRaces => "between_races",
    }
}

fn parse_join(text: &str) -> Option<JoinInProgress> {
    [JoinInProgress::Closed, JoinInProgress::FreeRoam, JoinInProgress::BetweenRaces]
        .into_iter()
        .find(|policy| join_text(*policy) == text)
}

fn cars_text(restriction: &CarRestriction) -> String {
    match restriction {
        CarRestriction::Any => "any".to_owned(),
        CarRestriction::Cars(cars) => {
            let ids: Vec<String> = cars.iter().map(u32::to_string).collect();
            format!("cars:{}", ids.join(","))
        }
        CarRestriction::Class(class) => format!("class:{class}"),
        CarRestriction::PerformanceCap(cap) => format!("cap:{cap}"),
    }
}

fn parse_cars(text: &str) -> Option<CarRestriction> {
    if text == "any" {
        return Some(CarRestriction::Any);
    }
    if let Some(class) = text.strip_prefix("class:") {
        return class.parse().ok().map(CarRestriction::Class);
    }
    if let Some(cap) = text.strip_prefix("cap:") {
        return cap.parse().ok().map(CarRestriction::PerformanceCap);
    }
    let list = text.strip_prefix("cars:")?;
    let ids: Result<Vec<u32>, _> = list.split(',').map(|id| id.trim().parse::<u32>()).collect();
    List::new(ids.ok()?).ok().map(CarRestriction::Cars)
}
