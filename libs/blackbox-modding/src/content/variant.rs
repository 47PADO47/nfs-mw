//! What every effect variant has in common (`docs/modding/effect-variants.md`): the option a mod adds values to, the folder of
//! the mod's `files/` that holds the profile of each value, and the profile file's name and format number.
//!
//! A *kind* of effect (the nitrous, the tire smoke) has an option in the gameplay settings with one value, `original`, which
//! shows only when a mod adds another. The value's id names `<folder>/<ID>/profile.toml` in the mod's files; the game reads it
//! through the install's layers when the player has chosen the value. Each kind has its own profile schema (a module of this
//! folder, with a `parse` function); a new kind adds one [`VariantKind`] constant here.

use super::ContentError;

/// The name of a variant's profile file.
pub const PROFILE_FILE: &str = "profile.toml";
/// The only format number this version reads, in every kind's profile.
pub const FORMAT: u32 = 1;

/// One kind of effect that mods can add variants of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantKind {
    /// The id of the option in the gameplay settings (`bb.settings`): `nitrous_variant`.
    pub option: &'static str,
    /// The folder under a mod's `files/` that holds the variants' files: `NITROUS`.
    pub folder: &'static str,
}

impl VariantKind {
    /// The path of the profile of variant `value` in the install's layout (the overlay of the mod's `files/` folder).
    pub fn profile_path(&self, value: &str) -> String {
        format!("{}/{}/{PROFILE_FILE}", self.folder, value.to_ascii_uppercase())
    }
}

/// The nitrous variants (`nitrous_fx`).
pub const NITROUS: VariantKind = VariantKind { option: "nitrous_variant", folder: "NITROUS" };
/// The tire smoke variants (`tire_smoke`).
pub const TIRE_SMOKE: VariantKind = VariantKind { option: "tire_smoke_variant", folder: "TIRESMOKE" };

/// The collision spark variants (`spark_variant`): the particle groups of the Original PC spark style.
pub const COLLISION_SPARKS: VariantKind = VariantKind { option: "collision_spark_variant", folder: "SPARKS" };
/// The restored spark variants (`xenon_variant`): the line sparks of the Restored (Experimental) spark style.
pub const XENON_SPARKS: VariantKind = VariantKind { option: "xenon_spark_variant", folder: "XENON" };

/// The wind variants (`wind_fx`): the loops that play with the car's speed.
pub const WIND: VariantKind = VariantKind { option: "wind_variant", folder: "WIND" };

/// Refuse a profile whose `format` is not the one this version reads.
pub(super) fn check_format(format: u32) -> Result<(), ContentError> {
    if format != FORMAT {
        return Err(ContentError::Field { field: "format", reason: "only format 1 is read" });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_lives_in_the_upper_case_folder_of_its_value() {
        assert_eq!(NITROUS.profile_path("mw2012"), "NITROUS/MW2012/profile.toml");
        assert_eq!(TIRE_SMOKE.profile_path("MyMod"), "TIRESMOKE/MYMOD/profile.toml");
    }

    #[test]
    fn the_kinds_have_distinct_options_and_folders() {
        let kinds = [NITROUS, TIRE_SMOKE, COLLISION_SPARKS, XENON_SPARKS, WIND];
        for (at, kind) in kinds.iter().enumerate() {
            for other in &kinds[at + 1..] {
                assert_ne!(kind.option, other.option);
                assert_ne!(kind.folder, other.folder);
            }
        }
        assert_eq!(COLLISION_SPARKS.profile_path("mw2012"), "SPARKS/MW2012/profile.toml");
        assert_eq!(XENON_SPARKS.profile_path("mw2012"), "XENON/MW2012/profile.toml");
        assert_eq!(WIND.profile_path("mw2012"), "WIND/MW2012/profile.toml");
    }

    #[test]
    fn only_format_one_is_read() {
        assert!(check_format(1).is_ok());
        assert!(check_format(2).is_err());
    }
}
