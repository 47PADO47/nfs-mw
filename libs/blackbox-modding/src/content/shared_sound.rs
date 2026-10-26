//! `[car_sound.shared]`: the recordings of the sounds that every car of a game shares, which a mod car plays in place of
//! the game's own class-wide banks (docs/modding/car-sound.md, "The shared sounds").
//!
//! A mod that converts a whole set of cars (the 2012 cars) writes these recordings once, in a folder of its own, and
//! every car's manifest names them by path, so no car holds a copy. A list left out is a sound the car does not give:
//! the game plays what it plays for a car without it.
//!
//! The files are 16-bit PCM WAV, mono or stereo, in the mod's `files/` folder under the install's paths (`SOUND/...`).
//! A list with several files is variants: one is picked at random each time.

use serde::Deserialize;

use super::ContentError;
use super::engine_sound::{MAX_EFFECT_FILES, is_sound_path};

/// A car's recordings of the shared sounds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SharedSound {
    /// Body hits of a light, a medium and a heavy blow: the car against another car, a wall or a prop that does not fall
    /// over. The blow's class is picked from its strength.
    pub impact_small: Vec<String>,
    pub impact_medium: Vec<String>,
    pub impact_big: Vec<String>,
    /// One-shots that repeat while the body scrapes along a surface.
    pub scrape: Vec<String>,
    /// Short bangs from the exhaust that follow a backfire. They stand in for `[car_sound] pops` when that list is empty.
    pub pops: Vec<String>,
}

impl SharedSound {
    /// Whether no list names a file.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct RawShared {
    #[serde(default)]
    impact_small: Vec<String>,
    #[serde(default)]
    impact_medium: Vec<String>,
    #[serde(default)]
    impact_big: Vec<String>,
    #[serde(default)]
    scrape: Vec<String>,
    #[serde(default)]
    pops: Vec<String>,
}

/// Check the table: at most 16 files to a list, each a path of the install's layout that names a `.WAV` file.
pub(super) fn check(raw: RawShared) -> Result<SharedSound, ContentError> {
    let list = |name: &'static str, files: Vec<String>| -> Result<Vec<String>, ContentError> {
        if files.len() > MAX_EFFECT_FILES {
            return Err(ContentError::Field { field: name, reason: "a list names at most 16 files" });
        }
        if let Some(bad) = files.iter().find(|file| !is_sound_path(file)) {
            return Err(ContentError::InvalidText(bad.clone()));
        }
        Ok(files)
    };
    Ok(SharedSound {
        impact_small: list("car_sound.shared.impact_small", raw.impact_small)?,
        impact_medium: list("car_sound.shared.impact_medium", raw.impact_medium)?,
        impact_big: list("car_sound.shared.impact_big", raw.impact_big)?,
        scrape: list("car_sound.shared.scrape", raw.scrape)?,
        pops: list("car_sound.shared.pops", raw.pops)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::engine_sound::{CarEffects, RawEffects, check_effects};

    #[derive(Deserialize)]
    struct File {
        car_sound: Option<RawEffects>,
    }

    fn effects(text: &str) -> Result<Option<CarEffects>, ContentError> {
        let file: File = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
        check_effects(file.car_sound)
    }

    #[test]
    fn a_car_with_only_shared_sounds_has_effects() {
        let text = "[car_sound.shared]\nimpact_big = [\"SOUND/S/B0.WAV\", \"SOUND/S/B1.WAV\"]\nscrape = [\"SOUND/S/R.WAV\"]\n";
        let got = effects(text).unwrap().unwrap();
        assert_eq!(got.shared.impact_big, ["SOUND/S/B0.WAV", "SOUND/S/B1.WAV"]);
        assert_eq!(got.shared.scrape, ["SOUND/S/R.WAV"]);
        assert!(got.shared.impact_small.is_empty() && got.shared.pops.is_empty());
        assert!(got.shift.is_empty(), "the car's own lists are separate");
    }

    #[test]
    fn the_shared_table_sits_beside_the_car_s_own_lists() {
        let text = "[car_sound]\nshift = [\"SOUND/A/S.WAV\"]\n[car_sound.shared]\npops = [\"SOUND/S/P.WAV\"]\n";
        let got = effects(text).unwrap().unwrap();
        assert_eq!((got.shift.len(), got.shared.pops.len()), (1, 1));
        assert!(effects("[car_sound.shared]\nscrape = []\n").unwrap().is_none(), "no file, no table");
    }

    #[test]
    fn bad_shared_tables_are_refused() {
        let seventeen = format!("[car_sound.shared]\nscrape = [{}]\n", vec!["\"SOUND/P.WAV\""; 17].join(", "));
        for (edit, text) in [
            ("a path with ..", "[car_sound.shared]\nimpact_small = [\"SOUND/../S.WAV\"]\n"),
            ("a file that is not a WAV", "[car_sound.shared]\nscrape = [\"SOUND/S.MP3\"]\n"),
            ("an absolute path", "[car_sound.shared]\npops = [\"/SOUND/P.WAV\"]\n"),
            ("seventeen files", seventeen.as_str()),
            ("a key the game does not know", "[car_sound.shared]\nskid = [\"SOUND/S.WAV\"]\n"),
            ("a file instead of a list", "[car_sound.shared]\nscrape = \"SOUND/S.WAV\"\n"),
        ] {
            assert!(effects(text).is_err(), "{edit}");
        }
    }
}
