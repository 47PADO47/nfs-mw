//! The wrong-way notice: shown when a racer drives against the route for a while, cleared when it turns back. The
//! server decides and sends it with the HUD state; the race goes on (no automatic reset in multiplayer).
//!
//! The input is [`heading_dot`]: the cosine between the car's velocity and the route's direction at the car (the
//! caller takes the route direction from its road network). Values **[ours]**: wrong when the cosine is below
//! [`WRONG_WAY_DOT`] at [`WRONG_WAY_MIN_SPEED`] or more for [`WRONG_WAY_SHOW_TICKS`]; cleared after
//! [`WRONG_WAY_CLEAR_TICKS`] the right way. Below the speed the state holds (a car stopped to turn round keeps it).

use nfsmw_sim::TICK_HZ;

/// Below this cosine between velocity and route the car goes the wrong way (more than about 100 degrees off).
pub const WRONG_WAY_DOT: f32 = -0.2;
/// The speed under which the heading is not judged, m/s.
pub const WRONG_WAY_MIN_SPEED: f32 = 3.0;
/// Ticks the wrong way before the notice shows: two seconds.
pub const WRONG_WAY_SHOW_TICKS: u32 = 2 * TICK_HZ;
/// Ticks the right way before the notice clears: half a second.
pub const WRONG_WAY_CLEAR_TICKS: u32 = TICK_HZ / 2;

/// The cosine between `velocity` and `route_direction` (both world vectors), 0 when either is zero or not finite.
pub fn heading_dot(velocity: [f32; 3], route_direction: [f32; 3]) -> f32 {
    let dot = velocity[0] * route_direction[0] + velocity[1] * route_direction[1] + velocity[2] * route_direction[2];
    let lengths = length(velocity) * length(route_direction);
    if !dot.is_finite() || !lengths.is_finite() || lengths <= 0.0 {
        return 0.0;
    }
    (dot / lengths).clamp(-1.0, 1.0)
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// One racer's wrong-way state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WrongWay {
    wrong_ticks: u32,
    right_ticks: u32,
    showing: bool,
}

impl WrongWay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the notice shows.
    pub fn showing(&self) -> bool {
        self.showing
    }

    /// Feeds one tick: the heading cosine and the speed. Returns `Some(true)` when the notice appears and
    /// `Some(false)` when it clears.
    pub fn update(&mut self, dot: f32, speed: f32) -> Option<bool> {
        if !dot.is_finite() || !speed.is_finite() || speed < WRONG_WAY_MIN_SPEED {
            return None;
        }
        if dot < WRONG_WAY_DOT {
            self.right_ticks = 0;
            self.wrong_ticks = self.wrong_ticks.saturating_add(1);
            if self.showing || self.wrong_ticks < WRONG_WAY_SHOW_TICKS {
                return None;
            }
            self.showing = true;
            return Some(true);
        }
        self.wrong_ticks = 0;
        self.right_ticks = self.right_ticks.saturating_add(1);
        if !self.showing || self.right_ticks < WRONG_WAY_CLEAR_TICKS {
            return None;
        }
        self.showing = false;
        Some(false)
    }

    /// Clears the state (a reset or a respawn).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
