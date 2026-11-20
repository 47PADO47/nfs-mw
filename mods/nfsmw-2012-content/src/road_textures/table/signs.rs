//! Signs: the faces of the STOP and the speed limit signs.
//!
//! One row for each texture of the 2005 world that is replaced: its name hash, its name as the pack keeps it (cut to 23
//! characters), the recipe, the size of the replacement, the alpha (255: none) and the gain. The rows come from the 2005
//! world's texture list; `docs/modding/road-textures.md` has the table and what each look is made from.

use super::super::recipe::Entry;
use super::recipes::*;

pub static ENTRIES: &[Entry] = &[
    Entry::new(991615404, "SGN_STOP", &STOP_SIGN, 256, 256, 255, 1.00),
    Entry::new(2163501083, "SGN_SPEEDLIMIT50", &SPEED_50, 256, 256, 255, 1.00),
    Entry::new(2163501088, "SGN_SPEEDLIMIT55", &SPEED_55, 256, 256, 255, 1.00),
];
