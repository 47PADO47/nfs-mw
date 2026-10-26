//! `cars/<id>.toml`: a car a mod adds. The file name is the car id, and the car's files are in `files/CARS/<ID>/`.

use serde::Deserialize;

use super::engine_sound::{self, CarEffects, EngineSound};
use super::performance::{self, Performance};
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
    /// The stock car it copies, as the game names car folders. The car keeps that car's record, parts and physics.
    pub clone: String,
    pub role: Role,
    /// The heat levels whose cop waves may use the car: ascending, no repeats. Empty when the list was left out, which
    /// the game reads as every level. Always empty for a car that is not a cop.
    pub heat: Vec<u32>,
    /// The display name, trimmed. `None` keeps the stock name of the clone.
    pub name: Option<String>,
    /// The brand, trimmed. `None` keeps the clone's brand.
    pub manufacturer: Option<String>,
    /// How the car drives compared with the clone (`[performance]`): the clone's own values when the table is left out.
    pub performance: Performance,
    /// The car's own engine loops (`[engine_sound]`), played instead of its clone's engine. `None` keeps the clone's.
    pub engine_sound: Option<EngineSound>,
    /// The car's own recordings of its shifts, turbo, backfire and pops (`[car_sound]`). A sound it does not give is silent.
    pub car_sound: Option<CarEffects>,
}

/// The file as serde reads it. Unknown keys are an error, so a key the game does not support (an `[override]`
/// table, say) refuses the car with the key named in the reason, rather than being ignored.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    clone: String,
    #[serde(default)]
    role: RawRole,
    #[serde(default)]
    heat: Option<Vec<u32>>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    performance: Option<performance::Raw>,
    #[serde(default)]
    engine_sound: Option<engine_sound::Raw>,
    #[serde(default)]
    car_sound: Option<engine_sound::RawEffects>,
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
    if !is_id(&raw.clone) {
        return Err(ContentError::InvalidClone(raw.clone));
    }
    let role = Role::from(raw.role);
    let heat = heat(raw.heat, role)?;
    let name = checked_text(raw.name, ContentError::InvalidName)?;
    let manufacturer = checked_text(raw.manufacturer, ContentError::InvalidManufacturer)?;
    let performance = performance::check(raw.performance)?;
    let engine_sound = engine_sound::check(raw.engine_sound)?;
    let car_sound = engine_sound::check_effects(raw.car_sound)?;
    Ok(CarManifest {
        id: stem.to_ascii_uppercase(),
        clone: raw.clone,
        role,
        heat,
        name,
        manufacturer,
        performance,
        engine_sound,
        car_sound,
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
