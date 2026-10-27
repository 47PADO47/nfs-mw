//! `bb.engine` on the host side: the backend the game implements, and the trampolines behind `BbEngine`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, emit_text, guard, slot_of, text_within, write_prefixed};
use crate::abi::{
    BB_ENGINE_OFF, BB_ENGINE_RUNNING, BB_ENGINE_STARTING, BB_ERR_ARG, BbEngine, BbEngineState, BbIface, BbSinkFn,
    BbStatus, BbStr, IFACE_ENGINE, IFACE_ENGINE_VERSION, MAX_PATH_BYTES,
};

/// The most gain `start` takes for its sound.
pub const MAX_START_GAIN: f32 = 8.0;
/// The longest start delay `start` takes, in seconds.
pub const MAX_START_DELAY: f32 = 10.0;

/// Where the player's engine is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnginePhase {
    /// Switched off.
    Off,
    /// Started, and running when the start delay is over.
    Starting,
    /// Running.
    #[default]
    Running,
}

/// The player's engine and pedals as the game reports them. Units as in [`BbEngineState`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EngineState {
    pub phase: EnginePhase,
    /// The player drives free of any event.
    pub free_roam: bool,
    /// The driver's throttle pedal, 0 to 1.
    pub throttle: f32,
    /// The driver's brake pedal, 0 to 1.
    pub brake: f32,
    /// Speed along the car, metres per second.
    pub speed: f32,
}

/// What the game implements for `bb.engine`. `owner` is the id of the mod that made the call.
pub trait EngineBackend {
    /// The engine and the pedals of the player's car. `NotReady` while no car is driven.
    fn state(&self, owner: &str) -> BackendResult<EngineState>;
    /// The id of the player's car. `NotReady` while no car is driven.
    fn car(&self, owner: &str) -> BackendResult<String>;
    /// Switch the engine off.
    fn stop(&self, owner: &str) -> BackendResult<()>;
    /// Start the engine: play `sound` (a path checked to be plain; empty for none) at `gain` now, and run after `delay`
    /// seconds. Both numbers are already checked to be finite and in range.
    fn start(&self, owner: &str, sound: &str, gain: f32, delay: f32) -> BackendResult<()>;
    /// The mod `owner` unloads: the game puts back what it changed (an engine the mod stopped runs again, with no sound).
    fn release(&self, _owner: &str) {}
}

/// Build the `bb.engine` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn EngineBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbEngine {
        head: BbIface {
            size: size_of::<BbEngine>() as u32,
            major: IFACE_ENGINE_VERSION.0,
            minor: IFACE_ENGINE_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        state: Some(state),
        car: Some(car),
        stop: Some(stop),
        start: Some(start),
    });
    // An engine the mod stopped must not stay off when the mod is gone: the release tells the backend.
    let kept = slot.clone();
    let release = Box::new(move || kept.backend.release(&kept.owner));
    Attached::new(IFACE_ENGINE, IFACE_ENGINE_VERSION, table, release)
}

/// A sound path a mod sends: plain relative text of at most [`MAX_PATH_BYTES`], with no `..`, no drive and no backslash.
/// The empty path is allowed and means no sound.
pub(crate) fn plain_path(path: &str) -> bool {
    if path.is_empty() {
        return true;
    }
    let bad_start = path.starts_with('/') || path.starts_with('.');
    let bad_byte = |byte: char| byte == '\\' || byte == ':' || byte.is_control();
    !bad_start && !path.contains("..") && !path.chars().any(bad_byte)
}

fn phase_code(phase: EnginePhase) -> u32 {
    match phase {
        EnginePhase::Off => BB_ENGINE_OFF,
        EnginePhase::Starting => BB_ENGINE_STARTING,
        EnginePhase::Running => BB_ENGINE_RUNNING,
    }
}

/// `state`: write the engine and the pedals to the mod's struct, as far as its size word allows.
unsafe extern "C" fn state(ctx: *mut c_void, out: *mut BbEngineState) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn EngineBackend>(ctx)? };
        let engine = slot.backend.state(&slot.owner)?;
        let wire = BbEngineState {
            size: size_of::<BbEngineState>() as u32,
            phase: phase_code(engine.phase),
            free_roam: u32::from(engine.free_roam),
            throttle: engine.throttle,
            brake: engine.brake,
            speed: engine.speed,
        };
        // SAFETY: `out` is the mod's struct; `write_prefixed` checks its size word before it writes.
        unsafe { write_prefixed(out, &wire) }
    })
}

/// `car`: the player's car id goes to the sink.
unsafe extern "C" fn car(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn EngineBackend>(ctx)? };
        let id = slot.backend.car(&slot.owner)?;
        // SAFETY: the sink and its user pointer are the mod's, valid for this call.
        unsafe { emit_text(sink, sink_user, &id) }
    })
}

/// `stop`: switch the engine off.
unsafe extern "C" fn stop(ctx: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn EngineBackend>(ctx)? };
        slot.backend.stop(&slot.owner)
    })
}

/// `start`: a plain sound path (or none), a gain from 0 to [`MAX_START_GAIN`] and a delay from 0 to [`MAX_START_DELAY`].
unsafe extern "C" fn start(ctx: *mut c_void, sound: BbStr, gain: f32, delay: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn EngineBackend>(ctx)? };
        // SAFETY: the mod's path is valid for the call (ADR 0005 section 4).
        let sound = unsafe { text_within(sound, MAX_PATH_BYTES) }?;
        if !plain_path(sound) {
            return Err(BB_ERR_ARG);
        }
        if !(0.0..=MAX_START_GAIN).contains(&gain) || !(0.0..=MAX_START_DELAY).contains(&delay) {
            return Err(BB_ERR_ARG);
        }
        slot.backend.start(&slot.owner, sound, gain, delay)
    })
}
