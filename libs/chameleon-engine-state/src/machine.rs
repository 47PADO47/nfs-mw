//! The standstill auto-stop as a state machine.

use crate::params::EngineParams;

/// Where the engine is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    /// Running. `still` is how long the car has stood still.
    Running { still: f32 },
    /// Switched off by the rule.
    Off,
    /// Cranking after a press of the throttle; `left` seconds to go.
    Starting { left: f32 },
}

/// What a step reports.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Nothing changed.
    None,
    /// The engine just switched off.
    Stopped,
    /// The throttle was pressed with the engine off: the ignition starts, and the engine runs after `seconds`.
    Cranking { seconds: f32 },
    /// The cranking is over, the engine runs.
    Running,
    /// The rule stopped applying (a race, a pursuit) while the engine was off or starting: it runs at once, with no
    /// ignition sound.
    Resumed,
}

/// What the machine reads each step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Input {
    /// Seconds since the last step.
    pub dt: f32,
    /// The speed of the car, metres per second (either sign).
    pub speed: f32,
    /// The driver's throttle pedal, 0 to 1, whether or not the engine is running.
    pub throttle: f32,
    /// Whether the rule applies now (free driving: no race, no pursuit).
    pub active: bool,
}

/// The state of the standstill rule.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineMachine {
    params: EngineParams,
    phase: Phase,
}

impl EngineMachine {
    /// A machine with a running engine.
    pub fn new(params: EngineParams) -> Self {
        Self { params: params.sanitised(), phase: Phase::Running { still: 0.0 } }
    }

    pub fn params(&self) -> &EngineParams {
        &self.params
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Whether the engine drives the car (the cranking counts as not running).
    pub fn is_running(&self) -> bool {
        matches!(self.phase, Phase::Running { .. })
    }

    /// Put the machine back to a running engine with the clock at zero.
    pub fn reset(&mut self) {
        self.phase = Phase::Running { still: 0.0 };
    }

    /// One step of `input.dt` seconds.
    pub fn step(&mut self, input: Input) -> Event {
        let dt = match input.dt.is_finite() && input.dt > 0.0 {
            true => input.dt,
            false => 0.0,
        };
        if !input.active {
            let was_running = self.is_running();
            self.reset();
            return match was_running {
                true => Event::None,
                false => Event::Resumed,
            };
        }
        let pressed = input.throttle >= self.params.restart_throttle;
        match self.phase {
            Phase::Running { still } => {
                let standing = input.speed.abs() < self.params.still_speed && !pressed;
                let still = match standing {
                    true => still + dt,
                    false => 0.0,
                };
                if still < self.params.idle_seconds {
                    self.phase = Phase::Running { still };
                    return Event::None;
                }
                self.phase = Phase::Off;
                Event::Stopped
            }
            Phase::Off => {
                if !pressed {
                    return Event::None;
                }
                self.phase = Phase::Starting { left: self.params.crank_seconds };
                Event::Cranking { seconds: self.params.crank_seconds }
            }
            Phase::Starting { left } => {
                let left = left - dt;
                if left > 0.0 {
                    self.phase = Phase::Starting { left };
                    return Event::None;
                }
                self.phase = Phase::Running { still: 0.0 };
                Event::Running
            }
        }
    }
}
