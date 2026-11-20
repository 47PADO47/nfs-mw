//! Covers: the storm grate and the round cover.
//!
//! One row for each texture of the 2005 world that is replaced: its name hash, its name as the pack keeps it (cut to 23
//! characters), the recipe, the size of the replacement, the alpha (255: none) and the gain. The rows come from the 2005
//! world's texture list; `docs/modding/road-textures.md` has the table and what each look is made from.

use super::super::recipe::Entry;
use super::recipes::*;

pub static ENTRIES: &[Entry] = &[
    Entry::new(2438296449, "OBJ_STORMGRATE", &GRATE_FACE, 128, 256, 64, 1.00),
    Entry::new(202710057, "OBJ_STORMGRATE_NORMAL", &N_GRATE, 128, 256, 255, 1.00),
    Entry::new(1737107706, "OBJ_GASFILLCOVER", &MANHOLE_COVER, 256, 256, 64, 1.00),
    Entry::new(2523516802, "OBJ_GASFILLCOVER_NORMAL", &N_MANHOLE, 256, 256, 255, 1.00),
];
