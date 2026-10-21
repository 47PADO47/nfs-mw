//! The arrest scene the bust picks: one of the 26 `Arrest*` NIS bundles, by the perp's class and a random variant. The
//! rule is read from `speed.exe` (`docs/specs/busted-sequence.md` section 4.1). Only 16 of the 26 names are reachable
//! in the chooser: the `ArrestM04`, `ArrestF23`, `ArrestM07`, `ArrestM14` and `ArrestF14` families (and `ArrestM23`,
//! which has no bundle) are never the final scene. The world plays the picked scene as a NIS at the arrest shot
//! (`flow.rs`).

/// The variants the random roll picks from (0 to 7).
pub const VARIANTS: u8 = 8;

/// Classes 0 and 1: the first four variants, then the `b` names for variants 4, 5, 6 and the rest.
const LOW_CLASS: [&str; 4] = ["ArrestM01", "ArrestM16", "ArrestF02", "ArrestF18"];
const LOW_CLASS_B: [&str; 4] = ["ArrestM01b", "ArrestM16b", "ArrestF02b", "ArrestF18b"];

/// Classes 2, 3 and any other value: variants 0 to 6, then `ArrestF07b` for anything above 6.
const HIGH_CLASS: [&str; 7] =
    ["ArrestM06", "ArrestM19", "ArrestF06", "ArrestF07", "ArrestM06b", "ArrestM19b", "ArrestF06b"];
const HIGH_CLASS_OVER: &str = "ArrestF07b";

/// The arrest scene for a perp `class` and a `variant` (0 to 7; larger values take the last name of their class).
pub fn arrest_scene(class: u8, variant: u8) -> &'static str {
    match class {
        0 | 1 => low_class(variant),
        _ => high_class(variant),
    }
}

fn low_class(variant: u8) -> &'static str {
    match variant {
        0..=3 => LOW_CLASS[usize::from(variant)],
        4 => LOW_CLASS_B[0],
        5 => LOW_CLASS_B[1],
        6 => LOW_CLASS_B[2],
        _ => LOW_CLASS_B[3],
    }
}

fn high_class(variant: u8) -> &'static str {
    match HIGH_CLASS.get(usize::from(variant)) {
        Some(name) => name,
        None => HIGH_CLASS_OVER,
    }
}

/// The variant for a random roll in `0..1` (the roll is clamped): an even split of the 8 variants.
pub fn variant_from_roll(roll: f32) -> u8 {
    let scaled = (roll.clamp(0.0, 1.0) * f32::from(VARIANTS)) as u8;
    scaled.min(VARIANTS - 1)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn scenes(class: u8) -> Vec<&'static str> {
        (0..VARIANTS).map(|v| arrest_scene(class, v)).collect()
    }

    #[test]
    fn low_classes_pick_the_four_names_then_their_b_variants() {
        let expected = [
            "ArrestM01",
            "ArrestM16",
            "ArrestF02",
            "ArrestF18",
            "ArrestM01b",
            "ArrestM16b",
            "ArrestF02b",
            "ArrestF18b",
        ];
        assert_eq!(scenes(0), expected);
        assert_eq!(scenes(1), expected, "class 1 takes the same path as class 0");
    }

    #[test]
    fn high_classes_pick_the_seven_names_and_f07b_above_them() {
        let expected = [
            "ArrestM06",
            "ArrestM19",
            "ArrestF06",
            "ArrestF07",
            "ArrestM06b",
            "ArrestM19b",
            "ArrestF06b",
            "ArrestF07b",
        ];
        assert_eq!(scenes(2), expected);
        assert_eq!(scenes(3), expected, "class 3 takes the same path as class 2");
        assert_eq!(scenes(9), expected, "any other class takes the path of class 2");
    }

    #[test]
    fn a_variant_past_the_table_takes_the_last_name() {
        assert_eq!(arrest_scene(0, 200), "ArrestF18b");
        assert_eq!(arrest_scene(2, 200), "ArrestF07b");
    }

    #[test]
    fn sixteen_scenes_are_reachable_and_none_of_the_unreachable_families() {
        let reachable: BTreeSet<&str> = scenes(0).into_iter().chain(scenes(2)).collect();
        assert_eq!(reachable.len(), 16);
        for name in ["ArrestM04", "ArrestF23", "ArrestM07", "ArrestM14", "ArrestF14", "ArrestM23"] {
            assert!(!reachable.iter().any(|s| s.starts_with(name)), "{name} is not reachable");
        }
    }

    #[test]
    fn the_roll_splits_evenly_and_stays_in_range() {
        assert_eq!(variant_from_roll(0.0), 0);
        assert_eq!(variant_from_roll(0.13), 1);
        assert_eq!(variant_from_roll(0.99), 7);
        assert_eq!(variant_from_roll(1.0), 7, "a roll of exactly 1 is the last variant, not past it");
        assert_eq!(variant_from_roll(-3.0), 0);
        assert_eq!(variant_from_roll(7.0), 7);
    }

    #[test]
    fn the_same_roll_and_class_pick_the_same_scene() {
        let roll = 0.42;
        assert_eq!(arrest_scene(0, variant_from_roll(roll)), arrest_scene(0, variant_from_roll(roll)));
        assert_eq!(arrest_scene(0, variant_from_roll(0.42)), "ArrestF18", "0.42 is variant 3, the fourth of the four");
    }

    /// Each reachable name has its bundle in the install (`NIS/Scene_<name>_BundleB.bun`).
    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn every_reachable_scene_has_its_bundle() {
        let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
        let nis = std::path::PathBuf::from(root).join("NIS");
        for class in [0, 2] {
            for name in scenes(class) {
                let file = nis.join(format!("Scene_{name}_BundleB.bun"));
                assert!(file.is_file(), "{} is missing", file.display());
            }
        }
    }
}
