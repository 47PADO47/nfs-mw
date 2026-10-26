//! `cars/<id>.toml`: a car a mod adds. The file name is the car id, and the car's files are in `files/CARS/<ID>/`.

use serde::Deserialize;

use super::cop::{self, CopClass, CopWave};
use super::customization::{self, Customization};
use super::damage_parts::{self, DamagePart};
use super::engine_sound::{self, CarEffects, EngineSound};
use super::stats::{self, Stats};
use super::{ContentError, is_id};

/// Who may drive or chase with the car.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Listed among the cars the player can drive.
    Player,
    /// Drives in the cop waves of the heat levels in [`CarManifest::heat`].
    Cop,
    /// Both.
    Both,
}

impl Role {
    /// Whether the car is a player car.
    pub fn is_player(self) -> bool {
        matches!(self, Self::Player | Self::Both)
    }

    /// Whether the car is a cop car.
    pub fn is_cop(self) -> bool {
        matches!(self, Self::Cop | Self::Both)
    }
}

/// A checked `cars/<id>.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct CarManifest {
    /// The car id: the file name without `.toml`, in upper case. Its folder is `files/CARS/<id>/`.
    pub id: String,
    pub role: Role,
    /// The heat levels whose cop waves may use the car: ascending, no repeats. Empty when the list was left out, which
    /// the game reads as every level. Always empty for a car that is not a cop.
    pub heat: Vec<u32>,
    /// The cop car's wave entries, in level order: the levels of `heat` with the default numbers and the `[[cop]]` tables
    /// with theirs. Empty when the manifest names no level (the game then reads every level, with the default numbers).
    /// Always empty for a car that is not a cop.
    pub waves: Vec<CopWave>,
    /// What kind of police unit the car is (`cop_class`). Patrol for a car that is not a cop.
    pub cop_class: CopClass,
    /// The display name, trimmed. `None` shows the car id.
    pub name: Option<String>,
    /// The brand, trimmed. `None` shows no brand.
    pub manufacturer: Option<String>,
    /// What the car is (`[stats]`): its mass, engine, gearbox, tyres and brakes, in absolute units. The engine's generic
    /// car when the table is left out.
    pub stats: Stats,
    /// The car's own engine loops (`[engine_sound]`), `None` for a car with no loops of its own.
    pub engine_sound: Option<EngineSound>,
    /// The car's own paints, rims and other parts (`[customization]`). `None`: the car offers none of its own.
    pub customization: Option<Customization>,
    /// The car's own recordings of its shifts, turbo, backfire and pops (`[car_sound]`). A sound it does not give is silent.
    pub car_sound: Option<CarEffects>,
    /// The parts that swing on a hinge or fall off as the car is damaged (`[[damage_parts]]`). Empty: none.
    pub damage_parts: Vec<DamagePart>,
}

/// The file as serde reads it. Unknown keys are an error, so a key the game does not support (an `[override]`
/// table, say) refuses the car with the key named in the reason, rather than being ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default)]
    role: RawRole,
    #[serde(default)]
    heat: Option<Vec<u32>>,
    #[serde(default)]
    cop: Option<Vec<cop::RawWave>>,
    #[serde(default)]
    cop_class: Option<CopClass>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    stats: Option<stats::Raw>,
    #[serde(default)]
    engine_sound: Option<engine_sound::Raw>,
    #[serde(default)]
    customization: Option<customization::Raw>,
    car_sound: Option<engine_sound::RawEffects>,
    #[serde(default)]
    damage_parts: Option<Vec<damage_parts::Raw>>,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum RawRole {
    #[default]
    Player,
    Cop,
    Both,
}

impl From<RawRole> for Role {
    fn from(role: RawRole) -> Self {
        match role {
            RawRole::Player => Role::Player,
            RawRole::Cop => Role::Cop,
            RawRole::Both => Role::Both,
        }
    }
}

/// Parse `cars/<stem>.toml`. `stem` is the file name without its extension and becomes the car id.
pub fn parse(stem: &str, text: &str) -> Result<CarManifest, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    validate(stem, raw)
}

fn validate(stem: &str, raw: Raw) -> Result<CarManifest, ContentError> {
    if !is_id(stem) {
        return Err(ContentError::InvalidId(stem.to_owned()));
    }
    let role = Role::from(raw.role);
    let heat = heat(raw.heat, role)?;
    if !role.is_cop() && (raw.cop.is_some() || raw.cop_class.is_some()) {
        return Err(ContentError::HeatWithoutCop);
    }
    let waves = cop::check(&heat, raw.cop)?;
    let cop_class = raw.cop_class.unwrap_or_default();
    let name = checked_text(raw.name, ContentError::InvalidName)?;
    let manufacturer = checked_text(raw.manufacturer, ContentError::InvalidManufacturer)?;
    let stats = stats::check(raw.stats)?;
    let engine_sound = engine_sound::check(raw.engine_sound)?;
    let customization = customization::check(raw.customization)?;
    let car_sound = engine_sound::check_effects(raw.car_sound)?;
    let damage_parts = damage_parts::check(raw.damage_parts)?;
    Ok(CarManifest {
        id: stem.to_ascii_uppercase(),
        role,
        heat,
        waves,
        cop_class,
        name,
        manufacturer,
        stats,
        engine_sound,
        customization,
        car_sound,
        damage_parts,
    })
}

/// The heat levels. A list left out is no list (empty). A list needs a cop role, names only levels from 1, and has no
/// repeats.
fn heat(levels: Option<Vec<u32>>, role: Role) -> Result<Vec<u32>, ContentError> {
    let Some(mut levels) = levels else { return Ok(Vec::new()) };
    if !role.is_cop() {
        return Err(ContentError::HeatWithoutCop);
    }
    if levels.is_empty() {
        return Err(ContentError::EmptyHeat);
    }
    if let Some(&level) = levels.iter().find(|&&level| level == 0) {
        return Err(ContentError::InvalidHeat(level));
    }
    levels.sort_unstable();
    if let Some(pair) = levels.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(ContentError::RepeatedHeat(pair[0]));
    }
    Ok(levels)
}

/// Trim a text field and refuse one that is empty or has a control character. A left-out field stays `None`.
pub(super) fn checked_text(
    value: Option<String>,
    refuse: fn(String) -> ContentError,
) -> Result<Option<String>, ContentError> {
    let Some(text) = value else { return Ok(None) };
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return Err(refuse(text));
    }
    Ok(Some(trimmed.to_owned()))
}
