//! The loading tips of the career (`docs/specs/loading-screen.md` section 5): the keys of the 24 tips of the original,
//! and the names the install gives their text and picture.
//!
//! A tip `K` has the language labels `TIP_K_HEADER` and `TIP_K_DESC` (`LANGUAGES/Labels.bin`, English table) and the
//! picture `TIP_K` in `GLOBAL/DYNTEX.BIN`. The picture's name is also the handle `Loading_Tips.fng` gives its image
//! (`Tip_Safehouse.tga` there is `fe_hash_upper("TIP_SAFEHOUSE")`).

/// The tip keys, in the order of the table of `speed.exe` (index 0 to 26, three keys occur twice: `WORLDMAP`,
/// `ENGAGE_MECHANIC` and `STRIKEOUT`).
pub const TIPS: [&str; 24] = [
    "STRIKEOUT",
    "ENGAGE_MECHANIC",
    "WORLDMAP",
    "CUSTOMIZATION_SHOPS",
    "CARLOTS",
    "SMS",
    "UNLOCKS",
    "SPEEDBREAKER",
    "ROADBLOCKS",
    "SUVRAM",
    "CROSS",
    "HELICOPTER",
    "ROLLING_ROADBLOCK",
    "BOX_FORMATION",
    "PURSUIT_BREAKERS",
    "HIDING_SPOTS",
    "BUSTED",
    "RIVAL_CHALLENGES",
    "MARKER_REWARDS",
    "BLACKLIST",
    "BOUNTY",
    "CHALLENGE_SERIES",
    "RAP_SHEET",
    "SAFEHOUSE",
];

/// The name of the tip's picture, which is also the stem of the image resource the package draws it with.
pub fn picture_name(key: &str) -> String {
    format!("TIP_{key}")
}

/// The language label of the tip's title.
pub fn header_label(key: &str) -> String {
    format!("TIP_{key}_HEADER")
}

/// The language label of the tip's text.
pub fn description_label(key: &str) -> String {
    format!("TIP_{key}_DESC")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_key() {
        assert_eq!(picture_name("BOUNTY"), "TIP_BOUNTY");
        assert_eq!(header_label("BOUNTY"), "TIP_BOUNTY_HEADER");
        assert_eq!(description_label("BOUNTY"), "TIP_BOUNTY_DESC");
    }

    #[test]
    fn the_keys_are_distinct() {
        let mut keys = TIPS.to_vec();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), TIPS.len());
    }
}
