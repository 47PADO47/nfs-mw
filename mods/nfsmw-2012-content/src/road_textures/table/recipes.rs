//! The looks, made of pictures of the 2012 game. A recipe is a size-free description (fractions of the result); each row of the
//! tables gives the size and the brightness of the 2005 texture it replaces.
//!
//! The 2005 road is a set of strips 128 pixels wide and 1024 long: asphalt with a painted line in one place, crosswalk bars, a
//! sidewalk of slabs. The recipes below rebuild each strip from 2012 asphalt and 2012 paint, with the line where the 2005 strip
//! has it (measured on the strips: a white edge line in columns 2 to 5 of 128, a yellow one in columns 2 to 4, the second white
//! edge in columns 117 to 126, crosswalk bars about every 51 rows).

use super::super::layer::Layer;
use super::super::recipe::Recipe;
use super::sources::*;

/// A normal map's flat colour: straight up.
const FLAT: [u8; 3] = [128, 128, 255];

/// The asphalt over the whole result, in square tiles.
const ASPH_BASE: Layer = Layer::new(ASPHALT).tiles(1, 0);
/// The aggregate normal map over the whole result, in square tiles.
const ASPH_NORMAL_BASE: Layer = Layer::new(ASPHALT_NORMAL).tiles(1, 0);

/// One painted line: the worn white paint of the 2012 line texture, cut to the shape of its tiles, over the asphalt. `across` is
/// where it starts and `wide` how wide, as fractions of the strip.
const fn line(across: f32, wide: f32) -> Layer {
    Layer::new(WHITE_LINE).crop(10, 0, 104, 512).dst(across, 0.0, wide, 1.0).tiles(1, 0).keep_aspect().luma(30, 110)
}

/// The same line in dashes: about six along a strip, on for 57 % of each, the first starting 3 % in (measured on the 2005 strips).
const fn dashed_line(across: f32, wide: f32) -> Layer {
    line(across, wide).dash(0.17, 0.097, 0.033)
}

/// One painted yellow line: the same with the yellow paint, which is darker, so its mask starts lower.
const fn yellow_line(across: f32, wide: f32) -> Layer {
    Layer::new(YELLOW_LINE).crop(10, 0, 103, 512).dst(across, 0.0, wide, 1.0).tiles(1, 0).keep_aspect().luma(15, 70)
}

/// One crosswalk bar: the same paint turned along the strip. `down` is where it starts as a fraction of the strip's length.
const fn bar(down: f32) -> Layer {
    Layer::new(WHITE_LINE).crop(10, 0, 104, 512).turns(1).dst(0.06, down, 0.88, 0.0303).keep_aspect().luma(30, 110)
}

/// Asphalt, a strip with no paint.
pub static ASPH_STRIP: Recipe = Recipe { id: "asphalt", fill: [90, 90, 90], layers: &[ASPH_BASE] };
/// Asphalt with a white edge line.
pub static ASPH_STRIP_W: Recipe =
    Recipe { id: "asphalt, white edge", fill: [90, 90, 90], layers: &[ASPH_BASE, dashed_line(0.02, 0.035)] };
/// Asphalt with a yellow line on the edge (the centre line of the road, which is the edge of this strip).
pub static ASPH_STRIP_Y: Recipe =
    Recipe { id: "asphalt, centre line", fill: [90, 90, 90], layers: &[ASPH_BASE, yellow_line(0.02, 0.03)] };
/// Asphalt with a white line on both edges.
pub static ASPH_STRIP_WW: Recipe = Recipe {
    id: "asphalt, two edge lines",
    fill: [90, 90, 90],
    layers: &[ASPH_BASE, dashed_line(0.02, 0.03), line(0.91, 0.07)],
};
/// The normal map of every asphalt strip.
pub static N_ASPH: Recipe = Recipe { id: "asphalt normal", fill: FLAT, layers: &[ASPH_NORMAL_BASE] };

static CROSSWALK_LAYERS: [Layer; 21] = [
    ASPH_BASE,
    bar(0.00879),
    bar(0.05879),
    bar(0.10879),
    bar(0.15879),
    bar(0.20879),
    bar(0.25879),
    bar(0.30879),
    bar(0.35879),
    bar(0.40879),
    bar(0.45879),
    bar(0.50879),
    bar(0.55879),
    bar(0.60879),
    bar(0.65879),
    bar(0.70879),
    bar(0.75879),
    bar(0.80879),
    bar(0.85879),
    bar(0.90879),
    bar(0.95879),
];
/// A crosswalk: white bars across the asphalt.
pub static CROSSWALK: Recipe = Recipe { id: "crosswalk", fill: [90, 90, 90], layers: &CROSSWALK_LAYERS };

/// Sidewalk: a strip of the concrete slabs, a quarter of the slab picture's width (so the slabs keep their shape).
pub static SIDEWALK: Recipe =
    Recipe { id: "sidewalk slabs", fill: [100, 100, 100], layers: &[Layer::new(SLAB).crop(768, 0, 512, 4096)] };
pub static N_SIDEWALK: Recipe =
    Recipe { id: "sidewalk normal", fill: FLAT, layers: &[Layer::new(SLAB_NORMAL).crop(384, 0, 256, 2048)] };

/// Grass: the 2012 grass in square tiles.
pub static GRASS_FILL: Recipe =
    Recipe { id: "grass", fill: [60, 70, 40], layers: &[Layer::new(GRASS).crop(0, 0, 1024, 1024).tiles(1, 0)] };
pub static N_GRASS: Recipe =
    Recipe { id: "grass normal", fill: FLAT, layers: &[Layer::new(GRASS_NORMAL).crop(0, 0, 512, 512).tiles(1, 0)] };

/// Concrete ground: the slabs, a square of them.
pub static CONC: Recipe = Recipe {
    id: "concrete ground",
    fill: [100, 100, 100],
    layers: &[Layer::new(SLAB).crop(0, 0, 2048, 2048).tiles(1, 0)],
};
pub static N_CONC: Recipe =
    Recipe { id: "concrete normal", fill: FLAT, layers: &[Layer::new(SLAB_NORMAL).crop(0, 0, 1024, 1024).tiles(1, 0)] };

/// A concrete wall: a panel of the 2012 wall in the shape of the 2005 texture.
pub static WALL_PANEL: Recipe = Recipe {
    id: "concrete wall",
    fill: [120, 120, 118],
    layers: &[Layer::new(WALL).crop(0, 0, 512, 512).keep_aspect()],
};

/// The storm grate: its middle five columns of slots, in the 1 to 2 shape of the 2005 grate.
pub static GRATE_FACE: Recipe =
    Recipe { id: "storm grate", fill: [60, 60, 60], layers: &[Layer::new(GRATE).crop(64, 0, 128, 256)] };
pub static N_GRATE: Recipe =
    Recipe { id: "storm grate normal", fill: FLAT, layers: &[Layer::new(GRATE_NORMAL).crop(64, 0, 128, 256)] };

/// The round cover on the grey of the concrete round it.
pub static MANHOLE_COVER: Recipe =
    Recipe { id: "round cover", fill: [112, 110, 104], layers: &[Layer::new(MANHOLE).disc()] };
pub static N_MANHOLE: Recipe =
    Recipe { id: "round cover normal", fill: FLAT, layers: &[Layer::new(MANHOLE_NORMAL).disc()] };

/// The STOP sign: the octagon on the grey of the metal behind it; the black corners of the 2012 picture are clear.
pub static STOP_SIGN: Recipe =
    Recipe { id: "STOP sign", fill: [112, 114, 118], layers: &[Layer::new(STOP).crop(4, 7, 170, 170).luma(10, 32)] };

/// The face of the "50" sign of the 2012 picture, turned upright (it lies on its side in the picture) and centred in the square: the
/// sign is 166 x 89 pixels at (84, 103), which is 89 x 166 once turned, and fills the height of the square.
const SPEED_FACE: Layer = Layer::new(SPEED_SIGNS).crop(84, 103, 166, 89).turns(3).dst(0.253, 0.04, 0.494, 0.92);

/// The speed limit 50: the 2012 face on the metal grey.
pub static SPEED_50: Recipe = Recipe { id: "speed limit 50", fill: [118, 120, 122], layers: &[SPEED_FACE] };

/// The speed limit 55: the 50 with the five of its first digit copied over the zero (the 2012 pictures have a 2, a 5 and a 0 only).
pub static SPEED_55: Recipe = Recipe {
    id: "speed limit 55",
    fill: [118, 120, 122],
    layers: &[SPEED_FACE, Layer::new(SPEED_SIGNS).crop(94, 105, 72, 44).turns(3).dst(0.4809, 0.501, 0.2445, 0.4)],
};
