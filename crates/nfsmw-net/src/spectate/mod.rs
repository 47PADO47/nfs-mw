//! Spectator rules (MP-41): who may watch, how often a spectator gets snapshots, and which car a spectator's camera
//! follows.
//!
//! - [`may_spectate`]: a chosen spectator needs a free spectator place (`max_spectators`, counted apart from the
//!   racers, MP-24); a racer eliminated from a knockout or busted out of a race always becomes one, outside the limit,
//!   because the player is already in the session.
//! - [`RateClass`] and [`snapshot_hz`]: a spectator gets snapshots at [`SPECTATOR_SNAPSHOT_HZ`] (a tuning value: the
//!   10 Hz of ADR 0006's reduced player record, and the 10 Hz acknowledgements of bandwidth.md case E) and sends no
//!   input; the server drops input frames from a spectator ([`sends_input`]).
//! - [`FollowCam`]: the spectator's camera: free, following a slot, or following the race leader. A target that leaves
//!   or is eliminated hands the camera to the next racer in slot order.

#[cfg(test)]
mod tests;

use crate::message::{Role, Settings};
use crate::version::RejectReason;

/// Snapshots per second for a spectator (tuning).
pub const SPECTATOR_SNAPSHOT_HZ: u8 = 10;

/// Why a member is a spectator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpectateReason {
    /// The player chose to watch, or the client is spectate-only (`CAP_SPECTATE_ONLY`).
    Chosen,
    /// The player joined a running race and waits for the next one (MP-23).
    WaitingForRace,
    /// The player was knocked out, or lost the race by a bust, and watches the rest.
    Eliminated,
}

/// Whether a member may become a spectator with `spectators` already watching.
pub fn may_spectate(settings: &Settings, spectators: usize, reason: SpectateReason) -> Result<(), RejectReason> {
    if reason == SpectateReason::Eliminated {
        return Ok(());
    }
    if spectators >= usize::from(settings.max_spectators) {
        return Err(RejectReason::ServerFull);
    }
    Ok(())
}

/// How a connection is served snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateClass {
    Player,
    Spectator,
}

impl RateClass {
    /// The class of a member with `role`.
    pub fn of(role: Role) -> Self {
        match role {
            Role::Spectator => Self::Spectator,
            Role::Racer | Role::Cop => Self::Player,
        }
    }
}

/// The snapshot rate of `class` when players get `player_hz`.
pub fn snapshot_hz(class: RateClass, player_hz: u8) -> u8 {
    match class {
        RateClass::Player => player_hz,
        RateClass::Spectator => player_hz.min(SPECTATOR_SNAPSHOT_HZ),
    }
}

/// Whether the server accepts input frames from a member with `role`.
pub fn sends_input(role: Role) -> bool {
    role != Role::Spectator
}

/// What a spectator's camera looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Camera {
    /// A free camera the client flies.
    Free,
    /// The car of this slot.
    Follow(u8),
    /// Whoever leads the race.
    #[default]
    Leader,
}

/// A spectator's camera choice, kept valid as racers come and go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FollowCam {
    camera: Camera,
}

impl FollowCam {
    /// A camera on the leader.
    pub fn new() -> Self {
        Self::default()
    }

    /// The choice.
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// Chooses `camera`. Following a slot that is not in `targets` (the racers in slot order) is refused.
    pub fn set(&mut self, camera: Camera, targets: &[u8]) -> bool {
        if let Camera::Follow(slot) = camera
            && !targets.contains(&slot)
        {
            return false;
        }
        self.camera = camera;
        true
    }

    /// The next racer after the followed one, wrapping; from a free or leader camera, the first racer.
    pub fn next(&mut self, targets: &[u8]) {
        self.step(targets, 1);
    }

    /// The racer before the followed one, wrapping.
    pub fn previous(&mut self, targets: &[u8]) {
        self.step(targets, targets.len().saturating_sub(1));
    }

    /// `slot` left or became a spectator: a camera that followed it moves to the next racer in slot order after it, or
    /// to the leader when no racer is left.
    pub fn target_left(&mut self, slot: u8, targets: &[u8]) {
        if self.camera != Camera::Follow(slot) {
            return;
        }
        let next = targets.iter().copied().find(|other| *other > slot).or_else(|| targets.first().copied());
        self.camera = next.map_or(Camera::Leader, Camera::Follow);
    }

    /// The slot to look at: the followed one, or `order[0]` (the race order) for the leader; `None` for a free camera
    /// or an empty race.
    pub fn target(&self, order: &[u8]) -> Option<u8> {
        match self.camera {
            Camera::Free => None,
            Camera::Follow(slot) => Some(slot),
            Camera::Leader => order.first().copied(),
        }
    }

    fn step(&mut self, targets: &[u8], by: usize) {
        if targets.is_empty() {
            self.camera = Camera::Leader;
            return;
        }
        let at = match self.camera {
            Camera::Follow(slot) => targets.iter().position(|other| *other == slot),
            Camera::Free | Camera::Leader => None,
        };
        let Some(at) = at else {
            self.camera = Camera::Follow(targets[0]);
            return;
        };
        self.camera = Camera::Follow(targets[(at + by) % targets.len()]);
    }
}
