//! The police car's icon on the minimap (`docs/specs/hud-cop-blips.md`): a plain white icon while the car is not in a
//! pursuit, and a light bar that cycles blue, grey, red, grey while it is.

/// The HUD element of cop icon `index` (0..[`ICON_COUNT`]): `MMICON_COPCAR_<index>`.
pub const ICON_COUNT: usize = 8;

pub fn icon_name(index: usize) -> String {
    format!("MMICON_COPCAR_{index}")
}

/// Steps in one full cycle of the light bar. The step counter runs 1..=8.
pub const CYCLE: u8 = 8;

const WHITE: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
const BLUE: [u8; 4] = [0x00, 0x00, 0xff, 0xff];
const RED: [u8; 4] = [0xa0, 0x00, 0x00, 0xff];
/// The original's `0xffcccccc`, shown between the blue and the red.
const GREY: [u8; 4] = [0xcc, 0xcc, 0xcc, 0xff];

/// The step of the light bar on frame `tick` (counted from 0): 1, 2, .. 8, then 1 again.
pub fn step_at(tick: u32) -> u8 {
    (tick % CYCLE as u32) as u8 + 1
}

/// The light bar's colour at `step`: steps 1 and 2 blue, 3 and 4 grey, 5 and 6 red, 7 and 8 grey.
pub fn pursuit_colour(step: u8) -> [u8; 4] {
    match step {
        1 | 2 => BLUE,
        5 | 6 => RED,
        _ => GREY,
    }
}

/// The icon's colour: white when the car is not pursuing, the light bar's colour when it is.
pub fn colour(pursuing: bool, step: u8) -> [u8; 4] {
    if !pursuing {
        return WHITE;
    }
    pursuit_colour(step)
}
