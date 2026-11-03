//! The police car's icon on the minimap (`docs/specs/hud-cop-blips.md`): a steady light grey icon while the car is not
//! pursuing (wrecked, or sent away), and a light bar that cycles blue, grey, red, grey while it is: nine steps,
//! one per look tick.

/// The HUD element of cop icon `index` (0..[`ICON_COUNT`]): `MMICON_COPCAR_<index>`.
pub const ICON_COUNT: usize = 8;

pub fn icon_name(index: usize) -> String {
    format!("MMICON_COPCAR_{index}")
}

/// Steps in one full cycle of the light bar. The step counter runs 0..=8, as the original's.
pub const CYCLE: u8 = 9;

/// The light bar's colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bar {
    Blue,
    Grey,
    Red,
}

const BLUE: [u8; 4] = [0x00, 0x00, 0xff, 0xff];
const RED: [u8; 4] = [0xa0, 0x00, 0x00, 0xff];
/// The original's `0xffcccccc`, shown between the blue and the red.
const GREY: [u8; 4] = [0xcc, 0xcc, 0xcc, 0xff];

/// The step of the light bar on look tick `tick` (counted from 0): 0, 1, .. 8, then 0 again.
pub fn step_at(tick: u32) -> u8 {
    (tick % CYCLE as u32) as u8
}

/// The light bar at `step`: three steps blue (0 to 2), one grey (3), three red (4 to 6), two grey (7 and 8).
pub fn bar(step: u8) -> Bar {
    match step % CYCLE {
        0..=2 => Bar::Blue,
        4..=6 => Bar::Red,
        _ => Bar::Grey,
    }
}

/// The light bar's colour at `step`.
pub fn pursuit_colour(step: u8) -> [u8; 4] {
    match bar(step) {
        Bar::Blue => BLUE,
        Bar::Red => RED,
        Bar::Grey => GREY,
    }
}

/// The icon's colour: the original's steady light grey when the car is not pursuing, the light bar's colour when it is.
pub fn colour(pursuing: bool, step: u8) -> [u8; 4] {
    if !pursuing {
        return GREY;
    }
    pursuit_colour(step)
}
