//! When a semi's 5th wheel lets go of its trailer.
//! Spec: `docs/specs/ai-traffic.md` (§2).

use glam::Vec3;

/// The joint is released when either body's up vector has a world-up component below this (it is on its side).
const MIN_UP_Y: f32 = 0.75;
/// The joint is released when the dot product of the two up vectors is below this (they fold or twist apart).
const MIN_UP_DOT: f32 = 0.8;
/// A body on fewer wheels than this is "off the ground".
const MIN_WHEELS: usize = 2;
/// The joint is released when either body has been off the ground for longer than this (seconds).
const MAX_OFF_GROUND: f32 = 2.0;

/// What the joint looks at of one body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Posture {
    /// The body's up direction in the world.
    pub up: Vec3,
    pub wheels_on_ground: usize,
}

/// Why the joint let go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// A body lies on its side.
    Tipped,
    /// The two bodies' up vectors diverge: the trailer folds against or twists off the tractor.
    Folded,
    /// A body has been on fewer than two wheels for too long.
    OffGround,
}

/// The 5th wheel's state: coupled, or let go for good.
#[derive(Debug, Clone, Copy, Default)]
pub struct Hitch {
    released: Option<Release>,
    /// Seconds each body (tractor, trailer) has been on fewer than two wheels.
    off_ground: [f32; 2],
}

impl Hitch {
    pub fn is_hitched(&self) -> bool {
        self.released.is_none()
    }

    /// Why the joint let go, if it did.
    pub fn released(&self) -> Option<Release> {
        self.released
    }

    /// Lets time `dt` pass with the bodies in these postures. Returns whether the joint is still coupled.
    pub fn update(&mut self, dt: f32, tractor: Posture, trailer: Posture) -> bool {
        if self.released.is_some() {
            return false;
        }
        for (time, body) in self.off_ground.iter_mut().zip([tractor, trailer]) {
            *time = match body.wheels_on_ground < MIN_WHEELS {
                true => *time + dt,
                false => 0.0,
            };
        }
        let tipped = tractor.up.y < MIN_UP_Y || trailer.up.y < MIN_UP_Y;
        let folded = tractor.up.dot(trailer.up) < MIN_UP_DOT;
        let airborne = self.off_ground.iter().any(|&t| t > MAX_OFF_GROUND);
        self.released = match (tipped, folded, airborne) {
            (true, _, _) => Some(Release::Tipped),
            (_, true, _) => Some(Release::Folded),
            (_, _, true) => Some(Release::OffGround),
            _ => None,
        };
        self.released.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL: Posture = Posture { up: Vec3::Y, wheels_on_ground: 4 };

    fn tilted(angle: f32) -> Posture {
        Posture { up: Vec3::new(angle.sin(), angle.cos(), 0.0), wheels_on_ground: 4 }
    }

    #[test]
    fn level_bodies_stay_coupled() {
        let mut hitch = Hitch::default();
        for _ in 0..600 {
            assert!(hitch.update(0.1, LEVEL, LEVEL));
        }
        assert!(hitch.is_hitched());
    }

    #[test]
    fn a_body_that_tips_past_the_limit_releases_the_joint() {
        // Both on their side together, so only the height of the up vectors tells: acos(0.75) is 41.4 degrees.
        let mut hitch = Hitch::default();
        assert!(hitch.update(0.1, tilted(0.70), tilted(0.70)), "40 degrees holds");
        assert!(!hitch.update(0.1, tilted(0.73), tilted(0.73)), "42 degrees lets go");
        assert!(!hitch.is_hitched());
    }

    #[test]
    fn one_body_leaning_far_from_the_other_releases_the_joint() {
        // 37 degrees from the other body: its up vector is still high, but the dot is 0.8 or less.
        let mut hitch = Hitch::default();
        assert!(hitch.update(0.1, tilted(0.60), LEVEL), "34 degrees holds");
        assert!(!hitch.update(0.1, tilted(0.66), LEVEL), "38 degrees lets go");
    }

    #[test]
    fn bodies_that_fold_apart_release_the_joint() {
        // Each leans 0.35 rad (20 degrees) the other way: each is upright enough, but the dot is 0.76.
        let mut hitch = Hitch::default();
        assert!(!hitch.update(0.1, tilted(0.35), tilted(-0.35)));
    }

    #[test]
    fn two_seconds_on_fewer_than_two_wheels_release_the_joint() {
        let mut hitch = Hitch::default();
        let flying = Posture { wheels_on_ground: 1, ..LEVEL };
        // Half seconds add up exactly.
        for _ in 0..4 {
            assert!(hitch.update(0.5, LEVEL, flying), "up to 2 s is allowed");
        }
        assert!(!hitch.update(0.5, LEVEL, flying));
    }

    #[test]
    fn touching_down_again_restarts_the_clock() {
        let mut hitch = Hitch::default();
        let flying = Posture { wheels_on_ground: 0, ..LEVEL };
        for _ in 0..15 {
            assert!(hitch.update(0.1, flying, LEVEL));
        }
        assert!(hitch.update(0.1, LEVEL, LEVEL));
        for _ in 0..15 {
            assert!(hitch.update(0.1, flying, LEVEL));
        }
    }

    #[test]
    fn a_released_joint_stays_released() {
        let mut hitch = Hitch::default();
        assert!(!hitch.update(0.1, tilted(1.5), LEVEL));
        assert!(!hitch.update(0.1, LEVEL, LEVEL));
    }
}
