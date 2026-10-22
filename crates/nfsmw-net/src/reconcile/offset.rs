//! The drawn correction offset (sync-model.md section 2.4).

use crate::geom::{self, IDENTITY, Quat, Vec3};
use crate::predict::Pose;

/// The part of past corrections not yet shown. The physics car always jumps to the corrected state; what is drawn is
/// the physics pose plus this offset, which decays to nothing with a half-life.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualOffset {
    position: Vec3,
    /// Applied before the physics orientation: drawn = `rotation * physics`.
    rotation: Quat,
}

impl Default for VisualOffset {
    fn default() -> Self {
        Self { position: [0.0; 3], rotation: IDENTITY }
    }
}

impl VisualOffset {
    /// Take in a correction that moved the physics pose from `before` to `after`, so the drawn pose does not move.
    pub fn absorb(&mut self, before: Pose, after: Pose) {
        self.position = geom::add(self.position, geom::sub(before.position, after.position));
        let drawn_before = geom::mul(self.rotation, before.orientation);
        self.rotation = geom::mul(drawn_before, geom::conjugate(after.orientation));
    }

    /// Let `dt_ms` of time pass: the offset shrinks by half every `half_life_ms`. A non-positive half-life clears it.
    pub fn decay(&mut self, dt_ms: f32, half_life_ms: f32) {
        if half_life_ms.is_nan() || half_life_ms <= 0.0 || !dt_ms.is_finite() {
            self.clear();
            return;
        }
        let keep = 0.5f32.powf(dt_ms.max(0.0) / half_life_ms);
        self.position = geom::scale(self.position, keep);
        self.rotation = geom::slerp(IDENTITY, self.rotation, keep);
        if geom::length(self.position) < 1.0e-5 && geom::angle_between(self.rotation, IDENTITY) < 1.0e-5 {
            self.clear();
        }
    }

    /// Drop the offset: the drawn pose jumps to the physics pose (a snap).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The drawn pose of the physics pose `physics`.
    pub fn apply(&self, physics: Pose) -> Pose {
        Pose {
            position: geom::add(physics.position, self.position),
            orientation: geom::mul(self.rotation, physics.orientation),
        }
    }

    /// The position part, metres.
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// The length of the position part, metres.
    pub fn distance(&self) -> f32 {
        geom::length(self.position)
    }

    /// The angle of the rotation part, radians.
    pub fn angle(&self) -> f32 {
        geom::angle_between(self.rotation, IDENTITY)
    }

    pub fn is_zero(&self) -> bool {
        self.position == [0.0; 3] && self.rotation == IDENTITY
    }
}
