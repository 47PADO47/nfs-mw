//! Co-op soft catch-up tether. Past the soft distance the trailing player is only offered a
//! "Catch up" warp behind the anchor. Nothing forces the warp.
//!
//! Positions are render space (Z-up: x forward, y left, z up; see `scenes::world::space`), the
//! space `Drive::position()` returns. The ground plane is x and y, so height never adds to distance.

use glam::{Vec2, Vec3};

/// The soft catch-up distance between co-op players, metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tether {
    pub soft: f32,
}

impl Default for Tether {
    fn default() -> Self {
        Self { soft: 500.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TetherState {
    Near,
    Prompt,
}

/// The ground-plane part of a render-space vector (x and y).
fn ground(v: Vec3) -> Vec2 {
    Vec2::new(v.x, v.y)
}

impl Tether {
    /// `Prompt` when the ground-plane distance between the two players is past the soft distance.
    pub fn judge(&self, anchor: Vec3, other: Vec3) -> TetherState {
        let distance = (ground(anchor) - ground(other)).length();
        if distance > self.soft {
            return TetherState::Prompt;
        }
        TetherState::Near
    }
}

/// A point `metres` behind `position`, along the ground-plane part of `forward` (render space).
/// The height of `position` is kept. Returns `position` when `forward` has no ground-plane length.
pub fn behind(position: Vec3, forward: Vec3, metres: f32) -> Vec3 {
    let heading = ground(forward).normalize_or_zero();
    if heading == Vec2::ZERO {
        return position;
    }
    let offset = heading * metres;
    Vec3::new(position.x - offset.x, position.y - offset.y, position.z)
}

/// The anchor is the first player in a pursuit or a race, else player 0.
pub fn anchor(busy: &[bool]) -> usize {
    busy.iter().position(|&b| b).unwrap_or(0)
}

/// Catch-up is offered only when the tether prompts and the trailing player is not a pursuit target.
pub fn may_catch_up(trailing_is_target: bool, state: TetherState) -> bool {
    if trailing_is_target {
        return false;
    }
    state == TetherState::Prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-3
    }

    #[test]
    fn near_inside_soft() {
        let tether = Tether::default();
        assert_eq!(tether.soft, 500.0);
        let anchor = Vec3::ZERO;
        assert_eq!(tether.judge(anchor, Vec3::new(300.0, 0.0, 0.0)), TetherState::Near);
        // Exactly on the soft distance is still near.
        assert_eq!(tether.judge(anchor, Vec3::new(0.0, 500.0, 0.0)), TetherState::Near);
        // Height does not count: the ground-plane distance is 0.
        assert_eq!(tether.judge(anchor, Vec3::new(0.0, 0.0, 900.0)), TetherState::Near);
    }

    #[test]
    fn prompt_past_soft() {
        let tether = Tether::default();
        let anchor = Vec3::ZERO;
        assert_eq!(tether.judge(anchor, Vec3::new(501.0, 0.0, 0.0)), TetherState::Prompt);
        assert_eq!(tether.judge(anchor, Vec3::new(0.0, -600.0, 0.0)), TetherState::Prompt);
        assert_eq!(tether.judge(anchor, Vec3::new(400.0, 400.0, 0.0)), TetherState::Prompt);
    }

    #[test]
    fn behind_is_opposite_heading() {
        let position = Vec3::new(10.0, 20.0, 3.0);
        // Only the ground-plane part of the heading counts; the height of the position is kept.
        let behind_x = behind(position, Vec3::new(1.0, 0.0, 0.5), 100.0);
        assert!(close(behind_x, Vec3::new(-90.0, 20.0, 3.0)));
        // A diagonal heading (3, 4) is normalised to (0.6, 0.8).
        let behind_diag = behind(Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0), 100.0);
        assert!(close(behind_diag, Vec3::new(-60.0, -80.0, 0.0)));
        // Straight up has no ground-plane length, so the position comes back.
        assert_eq!(behind(position, Vec3::Z, 100.0), position);
    }

    #[test]
    fn anchor_prefers_a_busy_player() {
        assert_eq!(anchor(&[false, true, true]), 1);
        assert_eq!(anchor(&[false, false]), 0);
        assert_eq!(anchor(&[]), 0);
    }

    #[test]
    fn no_catch_up_while_chased() {
        assert!(may_catch_up(false, TetherState::Prompt));
        assert!(!may_catch_up(false, TetherState::Near));
        assert!(!may_catch_up(true, TetherState::Prompt));
        assert!(!may_catch_up(true, TetherState::Near));
    }
}
