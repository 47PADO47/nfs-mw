//! The values this mod adds to the game's effect variant options (`bb.settings`, `docs/modding/effect-variants.md`).
//!
//! Each kind of effect with variants (the game's `modding::backends::variants::KINDS`) has an option with the value `original`;
//! this mod adds one value to each kind it converts. A kind is one [`Kind`] constant here and one entry of [`ALL`]; the files
//! of its value are written by the kind's own module (`tire_smoke.rs`, `spark_assets.rs`) under `files/<FOLDER>/MW2012/`. The
//! nitrous variant is added by `add_nitrous_variant` in `lib.rs`.

use blackbox_modding::sdk::{Ctx, Level, SettingValue};

/// One option the mod adds a value to.
pub struct Kind {
    /// The option's id (`tire_smoke_variant`) and label.
    pub option: &'static str,
    pub label: &'static str,
    /// The value's id, which names the folder of its files, and its label.
    pub value: &'static str,
    pub value_label: &'static str,
}

/// The tire smoke.
pub const TIRE_SMOKE: Kind = Kind {
    option: "tire_smoke_variant",
    label: "Tire smoke variant",
    value: crate::tire_smoke::VARIANT,
    value_label: crate::tire_smoke::LABEL,
};

/// The collision sparks of the Original PC spark style (`spark_assets.rs`).
pub const COLLISION_SPARKS: Kind = Kind {
    option: "collision_spark_variant",
    label: "Collision spark variant (Original PC)",
    value: crate::spark_assets::VARIANT,
    value_label: crate::spark_assets::LABEL,
};

/// The line sparks of the Restored (Experimental) spark style. The 2012 game has no wind contrail or speed lines, so this
/// variant is the sparks only.
pub const XENON_SPARKS: Kind = Kind {
    option: "xenon_spark_variant",
    label: "Restored spark variant (sparks only)",
    value: crate::spark_assets::VARIANT,
    value_label: crate::spark_assets::LABEL,
};

/// The wind of the speed (`wind_assets.rs`): the loops the 2012 game plays with the car's speed.
pub const WIND: Kind = Kind {
    option: "wind_variant",
    label: "Wind variant",
    value: crate::wind_assets::VARIANT,
    value_label: crate::wind_assets::LABEL,
};

/// The trees of the world (`trees.rs`): the models and textures of the 2012 game's trees in place of the 2005 game's. Offered
/// only while the mod's setting `trees` is on.
pub const TREES: Kind =
    Kind { option: "trees_variant", label: "Trees", value: crate::trees::VARIANT, value_label: crate::trees::LABEL };

/// The colour grading of the world (`grading.rs`): the cubes of the 2012 game's `ENVIRONMENT` bundle by hour and weather.
pub const GRADING: Kind = Kind {
    option: "grading_variant",
    label: "Color grading",
    value: crate::grading::VARIANT,
    value_label: crate::grading::LABEL,
};

/// Every kind this mod adds a value to, besides the nitrous and the trees.
pub const ALL: [&Kind; 5] = [&TIRE_SMOKE, &COLLISION_SPARKS, &XENON_SPARKS, &WIND, &GRADING];

/// Add the value of every kind, and the trees' when `trees` (the setting of that name) is on. A game without `bb.settings`
/// keeps its own effects.
pub fn add_all(ctx: &Ctx, trees: bool) {
    let Some(settings) = ctx.settings() else { return };
    let trees_kind: &[&Kind] = match trees {
        true => &[&TREES],
        false => &[],
    };
    for kind in ALL.iter().chain(trees_kind).copied() {
        let values = [SettingValue { id: kind.value, label: kind.value_label }];
        if let Err(error) = settings.add_option(kind.option, kind.label, &values) {
            ctx.log(Level::Warn, &format!("nfsmw-2012-content: the {} value is not added: {error}", kind.option));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_names_a_distinct_option_and_a_plain_value_id() {
        for (at, kind) in ALL.iter().enumerate() {
            assert!(kind.option.ends_with("_variant"));
            assert!(kind.value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'));
            assert!(ALL.iter().skip(at + 1).all(|other| other.option != kind.option));
        }
    }

    #[test]
    fn the_tire_smoke_option_is_the_games() {
        assert_eq!(TIRE_SMOKE.option, blackbox_modding::content::TIRE_SMOKE.option);
        assert_eq!(TIRE_SMOKE.value, "mw2012");
    }

    #[test]
    fn the_spark_options_are_the_games_and_name_the_folders_the_mod_writes() {
        use blackbox_modding::content::{COLLISION_SPARKS as GAME_SPARKS, XENON_SPARKS as GAME_XENON};
        assert_eq!((COLLISION_SPARKS.option, XENON_SPARKS.option), (GAME_SPARKS.option, GAME_XENON.option));
        let folder = |game_folder: &str, value: &str| format!("{game_folder}/{}", value.to_ascii_uppercase());
        assert_eq!(folder(GAME_SPARKS.folder, COLLISION_SPARKS.value), crate::spark_assets::SPARKS_FOLDER);
        assert_eq!(folder(GAME_XENON.folder, XENON_SPARKS.value), crate::spark_assets::XENON_FOLDER);
        assert!(XENON_SPARKS.label.contains("sparks only"), "the menu says what the variant leaves out");
    }

    #[test]
    fn the_trees_option_is_the_games_and_names_the_folder_the_mod_writes() {
        assert_eq!(TREES.option, blackbox_modding::content::TREES.option);
        let folder = format!("{}/{}", blackbox_modding::content::TREES.folder, TREES.value.to_ascii_uppercase());
        assert_eq!(folder, crate::trees::FOLDER);
        assert!(TREES.option.ends_with("_variant") && ALL.iter().all(|other| other.option != TREES.option));
    }

    #[test]
    fn the_grading_option_is_the_games_and_names_the_folder_the_mod_writes() {
        assert_eq!(GRADING.option, blackbox_modding::content::GRADING.option);
        let folder = format!("{}/{}", blackbox_modding::content::GRADING.folder, GRADING.value.to_ascii_uppercase());
        assert_eq!(folder, crate::grading::FOLDER);
    }

    #[test]
    fn the_wind_option_is_the_games_and_names_the_folder_the_mod_writes() {
        assert_eq!(WIND.option, blackbox_modding::content::WIND.option);
        let folder = format!("{}/{}", blackbox_modding::content::WIND.folder, WIND.value.to_ascii_uppercase());
        assert_eq!(folder, "WIND/MW2012");
    }
}
