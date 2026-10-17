//! The payloads the game sends the mods (ADR 0005 section 4; the wire numbers are in docs/modding/api.md).
//!
//! Each payload starts with its own `size`, the bytes the game filled in. The `FRAME_START` and `SCENE_TICK` events
//! are sent from the frame sets `ModsPre` and `ModsPost`.

use std::mem::size_of;

use blackbox_modding::abi::{BbFrameStart, BbInteriorEvent, BbModUnloading, BbSceneTick, BbStr};

/// `FRAME_START`: a frame begins, before the scene updates.
pub fn frame_start_payload(dt_real: f32) -> BbFrameStart {
    BbFrameStart { size: size_of::<BbFrameStart>() as u32, dt_real }
}

/// `SCENE_TICK`: the scene has updated. The simulated time is the real time scaled by the game speed setting, and
/// none passes while the game is paused. The scene's own clock is not read, so this is the setting's view of it.
pub fn scene_tick_payload(dt_real: f32, game_speed: f32, paused: bool) -> BbSceneTick {
    let dt_sim = match paused {
        true => 0.0,
        false => dt_real * game_speed,
    };
    BbSceneTick {
        size: size_of::<BbSceneTick>() as u32,
        dt_sim,
        time_scale: game_speed,
        paused: u8::from(paused),
        _pad: [0; 3],
    }
}

/// `MOD_UNLOADING`: the mod `mod_id` is about to unload. The payload borrows `mod_id`, so it must outlive the call.
pub fn unloading_payload(mod_id: &str) -> BbModUnloading {
    BbModUnloading { size: size_of::<BbModUnloading>() as u32, _pad: 0, mod_id: BbStr::new(mod_id) }
}

/// `INTERIOR_ENTERING`, `INTERIOR_ENTERED`, `INTERIOR_EXITING` and `INTERIOR_EXITED`: one step of a mod interior's entry or
/// exit. The payload borrows `interior`, so it must outlive the call.
pub fn interior_payload(interior: &str, position: [f32; 3]) -> BbInteriorEvent {
    BbInteriorEvent {
        size: size_of::<BbInteriorEvent>() as u32,
        _pad: 0,
        interior: BbStr::new(interior),
        position,
        _pad2: 0,
    }
}
