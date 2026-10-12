//! The blips the original game draws, as [`BlipStyle`]s: the cop, the racer, the checkpoint and the helicopter.
//! Their icons are the package's `MMICON_*` objects (`docs/formats/minimap.md`).

use super::blip_style::{BlipLook, BlipStyle, MapBlip};
use super::cop_blip;

pub const COP: &str = "cop";
pub const RACER: &str = "racer";
pub const CHECKPOINT: &str = "checkpoint";
pub const HELICOPTER: &str = "helicopter";

/// A police car: a white icon, and a light bar that cycles blue, grey and red while `active` (a pursuit).
pub struct Cop;

impl BlipStyle for Cop {
    fn name(&self) -> &'static str {
        COP
    }

    fn icons(&self) -> Vec<String> {
        (0..cop_blip::ICON_COUNT).map(cop_blip::icon_name).collect()
    }

    fn look(&self, blip: &MapBlip, tick: u32) -> BlipLook {
        BlipLook { colour: cop_blip::colour(blip.active, cop_blip::step_at(tick)), ..BlipLook::PLAIN }
    }
}

/// A racer in a race: `MMICON_AIRACER_0..7`.
pub struct Racer;

impl BlipStyle for Racer {
    fn name(&self) -> &'static str {
        RACER
    }

    fn icons(&self) -> Vec<String> {
        (0..8).map(|i| format!("MMICON_AIRACER_{i}")).collect()
    }
}

/// The race checkpoint. It shows only while `active` (the condition the race sets).
pub struct Checkpoint;

impl BlipStyle for Checkpoint {
    fn name(&self) -> &'static str {
        CHECKPOINT
    }

    fn icons(&self) -> Vec<String> {
        vec!["MMICON_CHECKPOINT".to_string()]
    }

    fn shows(&self, blip: &MapBlip) -> bool {
        blip.active
    }
}

/// The police helicopter.
pub struct Helicopter;

impl BlipStyle for Helicopter {
    fn name(&self) -> &'static str {
        HELICOPTER
    }

    fn icons(&self) -> Vec<String> {
        vec!["HELICOPTER_ICON_GROUP".to_string()]
    }
}

/// The styles the game uses.
pub fn builtin() -> Vec<Box<dyn BlipStyle>> {
    vec![Box::new(Cop), Box::new(Racer), Box::new(Checkpoint), Box::new(Helicopter)]
}
