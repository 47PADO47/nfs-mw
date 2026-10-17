//! Region-dependent presentation: the minimap and world map variant of a stage and the `Region` race attribute
//! (`docs/specs/career-regions.md` section 6).

/// The minimap and world map variant. One rule serves the free-roam minimap and the world map screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapVariant {
    /// Stage 13 and up: `MINI_MAP_UNLOCK_1.BIN`, `TRACK_MAP_UNLOCK_1`; the north-west quarter is lit.
    Unlock1,
    /// Stages 12 to 9: `MINI_MAP_UNLOCK_2.BIN`, `TRACK_MAP_UNLOCK_2`; all but the south-west is lit.
    Unlock2,
    /// Stage 8 and below: `MINI_MAP.BIN`, `TRACK_MAP`; everything is lit.
    Full,
}

impl MapVariant {
    /// The variant of a stage: 13 and up is `Unlock1`, 9 and up `Unlock2`, else `Full`. It reads the career settings
    /// byte outside a race (racing state 0), whatever the game mode.
    pub const fn of_stage(stage: u8) -> MapVariant {
        match stage {
            13.. => MapVariant::Unlock1,
            9..=12 => MapVariant::Unlock2,
            _ => MapVariant::Full,
        }
    }

    /// The minimap file under `TRACKS/L2RA/`.
    pub const fn minimap_file(self) -> &'static str {
        match self {
            MapVariant::Unlock1 => "MINI_MAP_UNLOCK_1.BIN",
            MapVariant::Unlock2 => "MINI_MAP_UNLOCK_2.BIN",
            MapVariant::Full => "MINI_MAP.BIN",
        }
    }

    /// The world map texture name in `TrackMaps.bin`.
    pub const fn world_map_texture(self) -> &'static str {
        match self {
            MapVariant::Unlock1 => "TRACK_MAP_UNLOCK_1",
            MapVariant::Unlock2 => "TRACK_MAP_UNLOCK_2",
            MapVariant::Full => "TRACK_MAP",
        }
    }

    /// The variant index used by the `Region` race attribute: `Unlock1` 0, `Unlock2` 1, `Full` 2.
    pub const fn index(self) -> u8 {
        match self {
            MapVariant::Unlock1 => 0,
            MapVariant::Unlock2 => 1,
            MapVariant::Full => 2,
        }
    }
}

/// The index of a race `Region` text: `college` 0, `coastal` 1, `city` 2 (the map variant index). Other text is
/// `None`.
pub fn region_index(text: &str) -> Option<u8> {
    match text {
        "college" => Some(0),
        "coastal" => Some(1),
        "city" => Some(2),
        _ => None,
    }
}

/// The value of the quick race region option that means "all regions".
pub const ALL_REGIONS: u8 = 3;

/// The quick race track list keeps a race only if its `Region` index equals the region stored in the profile quick
/// race options, or that value is 3 (all regions). (The race type must match too; that is the caller's.)
pub const fn quick_race_region_matches(race_region: u8, selected: u8) -> bool {
    selected == ALL_REGIONS || race_region == selected
}

/// The lit area of the three variants as 8 x 8 tile grids of mean brightness, row 0 at the top: `#` lit, `:` partly
/// lit, `.` black. Measured on the install; shown here as test data for the variant rule.
pub const LIT_GRID_UNLOCK_1: [&str; 8] =
    ["######:.", "#####:..", "#####...", "#####...", "#####...", "........", "........", "........"];
/// The `Unlock2` grid.
pub const LIT_GRID_UNLOCK_2: [&str; 8] =
    ["########", "########", "########", "########", "########", "....####", ".....##:", "........"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_variant_thresholds_are_13_and_9() {
        for stage in 1..=16u8 {
            let expected = match stage {
                13..=16 => MapVariant::Unlock1,
                9..=12 => MapVariant::Unlock2,
                _ => MapVariant::Full,
            };
            assert_eq!(MapVariant::of_stage(stage), expected, "stage {stage}");
        }
        assert_eq!(MapVariant::of_stage(13), MapVariant::Unlock1);
        assert_eq!(MapVariant::of_stage(12), MapVariant::Unlock2);
        assert_eq!(MapVariant::of_stage(9), MapVariant::Unlock2);
        assert_eq!(MapVariant::of_stage(8), MapVariant::Full);
    }

    #[test]
    fn the_variant_files_and_textures_are_the_documented_names() {
        assert_eq!(MapVariant::Unlock1.minimap_file(), "MINI_MAP_UNLOCK_1.BIN");
        assert_eq!(MapVariant::Unlock2.minimap_file(), "MINI_MAP_UNLOCK_2.BIN");
        assert_eq!(MapVariant::Full.minimap_file(), "MINI_MAP.BIN");
        assert_eq!(MapVariant::Unlock1.world_map_texture(), "TRACK_MAP_UNLOCK_1");
        assert_eq!(MapVariant::Unlock2.world_map_texture(), "TRACK_MAP_UNLOCK_2");
        assert_eq!(MapVariant::Full.world_map_texture(), "TRACK_MAP");
    }

    #[test]
    fn the_region_attribute_uses_the_variant_index() {
        assert_eq!(region_index("college"), Some(MapVariant::Unlock1.index()));
        assert_eq!(region_index("coastal"), Some(MapVariant::Unlock2.index()));
        assert_eq!(region_index("city"), Some(MapVariant::Full.index()));
        assert_eq!(region_index("downtown"), None);
        assert_eq!(region_index("College"), None);
    }

    #[test]
    fn the_quick_race_filter_accepts_the_selected_region_or_all() {
        assert!(quick_race_region_matches(1, 1));
        assert!(!quick_race_region_matches(1, 2));
        for race in 0..3 {
            assert!(quick_race_region_matches(race, ALL_REGIONS));
        }
    }

    #[test]
    fn the_lit_grids_grow_with_the_variant() {
        let lit = |grid: &[&str; 8]| grid.iter().flat_map(|row| row.chars()).filter(|&c| c == '#').count();
        assert!(lit(&LIT_GRID_UNLOCK_1) < lit(&LIT_GRID_UNLOCK_2));
        assert_eq!(lit(&LIT_GRID_UNLOCK_2), 5 * 8 + 4 + 2);
    }
}
