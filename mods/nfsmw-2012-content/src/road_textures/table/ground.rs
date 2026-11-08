//! The ground: grass, concrete and walls.
//!
//! One row for each texture of the 2005 world that is replaced: its name hash, its name as the pack keeps it (cut to 23
//! characters), the recipe, the size of the replacement, the alpha (255: none) and the gain. The rows come from the 2005
//! world's texture list; `docs/modding/road-textures.md` has the table and what each look is made from.

use super::super::recipe::Entry;
use super::recipes::*;

pub static ENTRIES: &[Entry] = &[
    Entry::new(2534867741, "MD_GRASSYTILE1024", &GRASS_FILL, 256, 2048, 64, 1.05),
    Entry::new(514432858, "TER_CAMP_GRASS1", &GRASS_FILL, 1024, 1024, 255, 1.20),
    Entry::new(359559541, "TRNS_GRASSA_LD_GRASSA_0", &GRASS_FILL, 512, 2048, 255, 0.70),
    Entry::new(280753722, "TRNS_GRASSA_LE_GRASSA_C", &GRASS_FILL, 256, 2048, 64, 0.97),
    Entry::new(280753723, "TRNS_GRASSA_LE_GRASSA_C", &GRASS_FILL, 256, 2048, 64, 0.92),
    Entry::new(280825596, "TRNS_GRASSA_LE_GRASSA_C", &GRASS_FILL, 512, 2048, 255, 0.70),
    Entry::new(280861533, "TRNS_GRASSA_LE_GRASSA_C", &GRASS_FILL, 256, 2048, 255, 0.70),
    Entry::new(914248837, "TRNS_GRASSA_LE_GRASSA_C", &N_GRASS, 256, 2048, 255, 1.00),
    Entry::new(2460617123, "TRNS_GRASSA_LE_GRASSA_C", &N_GRASS, 256, 2048, 255, 1.00),
    Entry::new(2791847106, "TRNS_GRASSA_LE_GRASSA_C", &N_GRASS, 256, 2048, 255, 1.00),
    Entry::new(1353660279, "TRNS_GRASSB_LD_GRASSB_0", &GRASS_FILL, 512, 2048, 255, 0.72),
    Entry::new(2347761017, "TRNS_GRASSC_LD_GRASSC_0", &GRASS_FILL, 512, 2048, 255, 0.61),
    Entry::new(1515753317, "TRN_TPARKGRASS1", &GRASS_FILL, 512, 512, 255, 1.05),
    Entry::new(122405584, "TRNS_GCGRASS_LD_FAREWAY", &GRASS_FILL, 256, 2048, 255, 0.78),
    Entry::new(1664841292, "TRNS_GCGRASS_LD_FAREWAY", &GRASS_FILL, 512, 2048, 255, 0.75),
    Entry::new(52015522, "TRNS_GCGRASS_LD_ROUGH_0", &GRASS_FILL, 512, 2048, 255, 0.55),
    Entry::new(831873447, "TRNS_GCGRASS_LE_ROUGH_C", &GRASS_FILL, 256, 2048, 255, 0.94),
    Entry::new(831981258, "TRNS_GCGRASS_LE_ROUGH_C", &GRASS_FILL, 256, 2048, 255, 0.65),
    Entry::new(3845718866, "TRNS_GCGRASS_LE_ROUGH_C", &N_GRASS, 256, 2048, 255, 1.00),
    Entry::new(2305690285, "TRNS_GCGRASS_LX_PUTTING", &GRASS_FILL, 512, 2048, 255, 1.00),
    Entry::new(53388557, "TRN_OLDCONCRETE_SIMPLE", &CONC, 1024, 1024, 64, 0.58),
    Entry::new(2643302453, "TRN_OLDCONCRETE_SIMPLE_", &N_CONC, 1024, 1024, 255, 1.00),
    Entry::new(396313784, "ARC_CAMPUS_MBWALL_CONC1", &WALL_PANEL, 512, 512, 255, 0.39),
    Entry::new(1127259950, "ARC_CAMPUS_MBWALL_CONC1", &WALL_PANEL, 512, 256, 255, 0.40),
    Entry::new(1127259951, "ARC_CAMPUS_MBWALL_CONC2", &WALL_PANEL, 512, 512, 255, 0.39),
    Entry::new(1240647992, "ARC_CAMPUS_MB_WALL_CONC", &WALL_PANEL, 512, 512, 255, 0.30),
    Entry::new(3047117742, "ARC_CAMPUS_MB_WALL_CONC", &WALL_PANEL, 512, 512, 64, 0.47),
];
