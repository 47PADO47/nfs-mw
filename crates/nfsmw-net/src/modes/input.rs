//! What the mode rules read: the world's events, one sample per player car per tick, and the session's own facts.

use nfsmw_sim::{EntityId, PlayerId, SimEvent};

/// One player's car as the server sees it at the end of a tick.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CarSample {
    pub player: PlayerId,
    pub entity: EntityId,
    /// World metres.
    pub position: [f32; 3],
    /// Metres per second.
    pub velocity: [f32; 3],
    /// Laps completed (0 in a sprint until the finish).
    pub laps: u32,
    /// Progress through the current lap or the sprint, 0..100.
    pub percent: f32,
    /// Visual damage, 0..1.
    pub damage: f32,
}

impl CarSample {
    /// The speed, m/s.
    pub fn speed(&self) -> f32 {
        length(self.velocity)
    }
}

/// The length of a 3-vector.
pub(crate) fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// The difference `a - b` of two 3-vectors.
pub(crate) fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// One fact a mode reads. The server feeds every input of a tick, then ends the tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModeInput {
    /// Something the world reported.
    Sim(SimEvent),
    /// A player car's state this tick.
    Car(CarSample),
    /// A player car entered speed trap `trap` (its index in the race's list) at `kmh`.
    TrapPass { player: PlayerId, trap: u8, kmh: f32 },
    /// The shared pursuit's heat level, as the world's pursuit rules set it.
    Heat { level: u8 },
    /// The server granted a reset ([`crate::fairness::ResetRule`]) with this time penalty.
    Reset { player: PlayerId, penalty_ms: u32 },
    /// The player left the session: quit, timed out or was kicked.
    Left { player: PlayerId },
}
