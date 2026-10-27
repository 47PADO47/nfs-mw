//! The numbers of the standstill rule.

/// The parameters of [`crate::EngineMachine`]. Every field is a design value unless its doc says it was read from the
/// install; see `docs/formats/chameleon-engine-state.md`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineParams {
    /// Seconds the car must stand still before the engine switches off **[unconfirmed]** (the user's "a few seconds").
    pub idle_seconds: f32,
    /// Below this speed (metres per second, either way) the car stands still **[unconfirmed]**.
    pub still_speed: f32,
    /// A throttle of at least this (0 to 1) starts an engine that is off, and keeps a running one from counting
    /// **[unconfirmed]**.
    pub restart_throttle: f32,
    /// Seconds from the press to the running engine. The default is the length of the Porsche's ignition sample
    /// (0.591 s, the window of its `Player Vehicle Ignition` sequence), see [`crate::Ignition::seconds`] **[verified for
    /// the Porsche; that the engine waits for it is unconfirmed]**.
    pub crank_seconds: f32,
}

impl Default for EngineParams {
    fn default() -> Self {
        Self { idle_seconds: 5.0, still_speed: 0.5, restart_throttle: 0.1, crank_seconds: 0.6 }
    }
}

impl EngineParams {
    /// The parameters with every number made usable: finite, not negative, the throttle in 0 to 1.
    pub fn sanitised(self) -> Self {
        let positive = |value: f32, fallback: f32| match value.is_finite() && value >= 0.0 {
            true => value,
            false => fallback,
        };
        let defaults = Self::default();
        Self {
            idle_seconds: positive(self.idle_seconds, defaults.idle_seconds),
            still_speed: positive(self.still_speed, defaults.still_speed),
            restart_throttle: positive(self.restart_throttle, defaults.restart_throttle).min(1.0),
            crank_seconds: positive(self.crank_seconds, defaults.crank_seconds),
        }
    }
}
