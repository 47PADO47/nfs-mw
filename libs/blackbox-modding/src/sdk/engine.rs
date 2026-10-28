//! `Engine`: read the player's pedals and switch the player's engine off and on (catalogue: docs/modding/api-engine.md).

use super::ModResult;
use super::ffi::engine::{self as host_engine, EngineFns};
use crate::abi::{BB_ENGINE_OFF, BB_ENGINE_STARTING, BbEngineState};

/// Where the player's engine is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnginePhase {
    /// Switched off: no throttle, no revs, no sound.
    Off,
    /// Started; it runs when the start delay is over.
    Starting,
    /// Running.
    Running,
}

/// The player's engine and pedals, as a mod reads them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineState {
    /// Where the engine is.
    pub phase: EnginePhase,
    /// Whether the player drives free of any event (no race, no pursuit, no cutscene, no menu over the car).
    pub free_roam: bool,
    /// The driver's throttle pedal, 0 to 1, whether or not the engine takes it.
    pub throttle: f32,
    /// The driver's brake pedal, 0 to 1.
    pub brake: f32,
    /// Speed along the car, metres per second (negative in reverse).
    pub speed: f32,
}

impl EngineState {
    fn from_abi(raw: BbEngineState) -> Self {
        let phase = match raw.phase {
            BB_ENGINE_OFF => EnginePhase::Off,
            BB_ENGINE_STARTING => EnginePhase::Starting,
            _ => EnginePhase::Running,
        };
        EngineState { phase, free_roam: raw.free_roam != 0, throttle: raw.throttle, brake: raw.brake, speed: raw.speed }
    }
}

/// The `bb.engine` interface of the host, as a mod uses it. Obtained from `Ctx::engine`.
#[derive(Clone, Copy)]
pub struct Engine {
    fns: EngineFns,
}

impl Engine {
    pub(crate) fn new(fns: EngineFns) -> Self {
        Engine { fns }
    }

    /// The engine and the pedals of the player's car. `NotReady` while no car is driven.
    pub fn state(&self) -> ModResult<EngineState> {
        host_engine::state(&self.fns).map(EngineState::from_abi)
    }

    /// The id of the player's car (`POR911CARR12`). `NotReady` while no car is driven.
    pub fn car(&self) -> ModResult<String> {
        host_engine::car(&self.fns)
    }

    /// Switch the engine off. It takes no throttle and no nitrous, shows no revs and makes no sound until it is started.
    pub fn stop(&self) -> ModResult<()> {
        host_engine::stop(&self.fns)
    }

    /// Start the engine. `sound` is a WAV path in the mods' `files/` layers (empty for none), played now at `gain` (0 to
    /// 8); the engine runs after `delay` seconds (0 to 10; 0 is at once).
    pub fn start(&self, sound: &str, gain: f32, delay: f32) -> ModResult<()> {
        host_engine::start(&self.fns, sound, gain, delay)
    }
}
