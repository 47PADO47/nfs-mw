//! A wet road profile: how a mod's variant wets the road (`docs/modding/effect-variants-marks-rain.md`).
//!
//! The file is `WETROAD/<value id>/profile.toml` in the mod's `files/` folder, where `<value id>` is the id of the value the mod
//! adds to the `wet_road_variant` option (`bb.settings`). The game wets its road with the rain: the dampness builds while it
//! rains and drains afterwards, the pools build after saturation, and the darkening and the reflections of the road follow
//! them. A variant changes how strongly and how fast: it scales the two levels the shading reads (the dampness and the pooling,
//! each kept to 0 to 1 afterwards) and the two rates at which the road gets wet and dries. When the rain falls is the game's.
//!
//! ```toml
//! format = 1
//! film = 1.2        # times the dampness of the road: how dark and how glossy a wet road is, 0 to 2 (default 1)
//! pooling = 1.0     # times the pooling: how much standing water there is on the road, 0 to 3 (default 1)
//! soak = 1.0        # times the rate at which the road gets wet, 0.25 to 4 (default 1)
//! dry = 1.0         # times the rate at which it dries (and the pools drain), 0.25 to 4 (default 1)
//! ```

use std::ops::RangeInclusive;

use serde::Deserialize;

use super::ContentError;
use super::variant::{self, WET_ROAD};

/// The folder under a mod's `files/` that holds the variants' files.
pub const WETROAD_FOLDER: &str = WET_ROAD.folder;

/// The path of the profile of variant `value` in the install's layout.
pub fn wet_road_profile_path(value: &str) -> String {
    WET_ROAD.profile_path(value)
}

/// A wet road variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WetRoadProfile {
    pub film: f32,
    pub pooling: f32,
    pub soak: f32,
    pub dry: f32,
}

impl Default for WetRoadProfile {
    /// The game's own wet road.
    fn default() -> Self {
        Self { film: 1.0, pooling: 1.0, soak: 1.0, dry: 1.0 }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    format: u32,
    film: Option<f32>,
    pooling: Option<f32>,
    soak: Option<f32>,
    dry: Option<f32>,
}

fn within(value: Option<f32>, range: &RangeInclusive<f32>, name: &'static str) -> Result<f32, ContentError> {
    let value = value.unwrap_or(1.0);
    if !value.is_finite() || !range.contains(&value) {
        return Err(ContentError::Field { field: name, reason: "a number outside the range this field takes" });
    }
    Ok(value)
}

/// Parse and check a profile. A number out of range, a key the schema does not know or another format number is refused with the
/// reason.
pub fn parse(text: &str) -> Result<WetRoadProfile, ContentError> {
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    variant::check_format(raw.format)?;
    Ok(WetRoadProfile {
        film: within(raw.film, &(0.0..=2.0), "film")?,
        pooling: within(raw.pooling, &(0.0..=3.0), "pooling")?,
        soak: within(raw.soak, &(0.25..=4.0), "soak")?,
        dry: within(raw.dry, &(0.25..=4.0), "dry")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_scales_the_four_numbers_and_leaves_out_the_game_s_own() {
        let profile = parse("format = 1\nfilm = 1.2\nsoak = 2.0").unwrap();
        assert_eq!(profile, WetRoadProfile { film: 1.2, pooling: 1.0, soak: 2.0, dry: 1.0 });
        assert_eq!(parse("format = 1").unwrap(), WetRoadProfile::default());
    }

    #[test]
    fn numbers_and_keys_are_checked() {
        assert!(matches!(parse("format = 2"), Err(ContentError::Field { field: "format", .. })));
        assert!(matches!(parse("film = 1.0"), Err(ContentError::Parse(_))));
        assert!(matches!(parse("format = 1\nfilm = 2.5"), Err(ContentError::Field { field: "film", .. })));
        assert!(matches!(parse("format = 1\npooling = -1.0"), Err(ContentError::Field { field: "pooling", .. })));
        assert!(matches!(parse("format = 1\nsoak = 0.0"), Err(ContentError::Field { field: "soak", .. })));
        assert!(matches!(parse("format = 1\ndry = 9.0"), Err(ContentError::Field { field: "dry", .. })));
        assert!(matches!(parse("format = 1\nwhat = 1"), Err(ContentError::Parse(_))));
    }

    #[test]
    fn the_profile_path_is_the_wet_road_folder_of_the_value() {
        assert_eq!(wet_road_profile_path("mw2012"), "WETROAD/MW2012/profile.toml");
        assert_eq!(WETROAD_FOLDER, "WETROAD");
    }
}
