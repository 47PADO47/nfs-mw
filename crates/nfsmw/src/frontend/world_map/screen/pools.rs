//! The objects of `WorldMapMain.fng` the map draws with, by GUID (`nfsmw dump-screen WorldMapMain.fng`, the names and
//! positions are in `docs/provenance/world-map-screen.md`). The blips take the pool objects of their style in the order
//! the pools are listed here, and the pools the map does not use stay parked off the picture.
//!
//! Which pool each style uses is from the executable's trace (`docs/provenance/world-map-ghidra.md`, verified there).
//! The blip's icon is set from its style, so the pool decides only where the blip is drawn.

use crate::hud::{
    AUTO_SHOP, CAR_LOT, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP, RACE_SPRINT, RACE_TOLLBOOTH, SAFE_HOUSE,
};

/// The map's picture object (`TRACK_MAP.tga`).
pub const PICTURE: u32 = 0xF9984;
/// The cursor's group: the glow and the four corners. It moves to the attached blip.
pub const CURSOR: u32 = 0xF93EF;
/// The card under the picture: its group, the title and the distance, and the card's mode icon.
pub const CARD: u32 = 0xF96AF;
pub const CARD_TITLE: u32 = 0xF570E;
pub const CARD_DISTANCE: u32 = 0xF5710;
pub const CARD_ICON: u32 = 0xFA3CE;
/// The tab's name ("LOCATIONS"), between the two arrows.
pub const TAB: u32 = 0xFA3A9;
/// The zoom readout under the picture ("ZOOM x 1.5" in the package).
pub const ZOOM: u32 = 0xF5707;

/// The pools of each blip style, in the order the blips take them. The executable finds each pool by the hash of its
/// name (`SHOP_%d`, `MMICON_SPRINT_%d`, ...; `docs/provenance/world-map-ghidra.md`), so the order is the order of the
/// names: the locations' pools are listed by their name's hash, the races' by their index.
pub const POOLS: [(&str, &[u32]); 9] = [
    (AUTO_SHOP, &[0xFA072, 0xFA073, 0xFA070, 0xFA071, 0x122C16, 0x122C1B, 0x122C20, 0x122C25]),
    (SAFE_HOUSE, &[0xF54A4, 0xF54A1, 0xF54A2, 0xF54A3]),
    (CAR_LOT, &[0xFA07C, 0xFA07D, 0xFA07A, 0xFA07B]),
    (RACE_SPRINT, &[0xF50D5, 0xF50E8, 0xF50ED, 0xF50F2, 0xF50F7, 0xF50FC]),
    (RACE_DRAG, &[0xF5129, 0xF5131, 0xF5136]),
    (RACE_CIRCUIT, &[0xF5088, 0xF5090, 0xF5095, 0xF509A, 0xF509F, 0xF50A4]),
    (RACE_KNOCKOUT, &[0xF51B9, 0xF51C2, 0xF51C7, 0xF51CC, 0xF51D1, 0xF51D6]),
    (RACE_TOLLBOOTH, &[0xF51FA, 0xF5201, 0xF5206]),
    (RACE_SPEEDTRAP, &[0x10502E, 0x10502F, 0x105030, 0x105031, 0x105032, 0x105033]),
];

/// The pools of a style (none for a style the map does not draw).
pub fn slots(style: &str) -> &'static [u32] {
    POOLS.iter().find(|(s, _)| *s == style).map_or(&[], |(_, slots)| *slots)
}

/// The rows of the blip list: the row's text, its icon group and the icon inside the group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub text: u32,
    pub icon_group: u32,
    pub icon: u32,
}

/// The seven rows of the list, from the top.
pub const ROWS: [Row; 7] = [
    Row { text: 0xF55E3, icon_group: 0xFC5E6, icon: 0xF55EF },
    Row { text: 0xFA8B0, icon_group: 0xFD2EC, icon: 0xFD2ED },
    Row { text: 0xFA8B3, icon_group: 0xFD2F0, icon: 0xFD2F1 },
    Row { text: 0xFA8B6, icon_group: 0xFD2F4, icon: 0xFD2F5 },
    Row { text: 0xFA8B9, icon_group: 0xFD2F8, icon: 0xFD2F9 },
    Row { text: 0xFA8BC, icon_group: 0xFD2FC, icon: 0xFD2FD },
    Row { text: 0xFA8BF, icon_group: 0xFD300, icon: 0xFD301 },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pool_object_is_listed_once() {
        let mut guids: Vec<u32> = POOLS.iter().flat_map(|(_, s)| s.iter().copied()).collect();
        let count = guids.len();
        guids.sort_unstable();
        guids.dedup();
        assert_eq!(guids.len(), count);
    }

    #[test]
    fn the_pool_sizes_match_the_package() {
        assert_eq!(slots(AUTO_SHOP).len(), 8);
        assert_eq!(slots(SAFE_HOUSE).len(), 4);
        assert_eq!(slots(CAR_LOT).len(), 4);
        assert_eq!(slots(RACE_SPRINT).len(), 6);
        assert!(slots("no_such_style").is_empty());
    }

    #[test]
    fn the_list_has_seven_rows() {
        assert_eq!(ROWS.len(), 7);
        let mut texts: Vec<u32> = ROWS.iter().map(|r| r.text).collect();
        texts.sort_unstable();
        texts.dedup();
        assert_eq!(texts.len(), 7);
    }
}
