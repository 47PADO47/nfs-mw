//! Reverse: a bike that stands with its brake held is walked backwards by its rider (`docs/specs/ride6-rider-styles.md`,
//! section 5).
//!
//! The rider holds the brake lever with the bike standing and no throttle. After a hold time the brakes are let go and a force
//! along the bike pushes it backwards, shrinking as the speed backwards nears the limit. The rider's feet are down; the bike has
//! no drive in reverse. A bike with at most two gears (a scooter) has no reverse.

use glam::Vec3;

use super::{Bike, BikeInput};

/// The lever and throttle values the input handler counts as pressed.
pub const PRESSED: f32 = 0.1;
/// The speed along the bike (m/s) below which a held brake asks for reverse.
pub const REQUEST_SPEED: f32 = 1.0;

/// The numbers of a bike's reverse (`[bike.reverse]` of the manifest).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReverseSpec {
    /// The force that pushes the bike backwards at a standstill with the lever fully held (N).
    pub force: f32,
    /// The speed backwards (m/s) at which the force has gone.
    pub max_speed: f32,
    /// How long the brake must be held with the bike standing before it rolls back (s).
    pub hold_time: f32,
}

impl ReverseSpec {
    /// Whether the numbers can run a reverse: all positive and finite.
    pub fn is_valid(&self) -> bool {
        [self.force, self.max_speed, self.hold_time].iter().all(|v| v.is_finite() && *v > 0.0)
    }
}

/// What the reverse keeps from step to step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReverseState {
    /// How long the brake has been held on a standing bike (s).
    timer: f32,
    /// Whether the rider is pushing the bike back in this step.
    pushing: bool,
}

/// What the reverse does in one step: the force along the bike (N, negative backwards) and whether the brakes are let go.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReverseDrive {
    pub force: f32,
    pub release: bool,
}

impl ReverseDrive {
    /// The brake levers after the reverse: let go while the rider pushes, as they were otherwise.
    pub fn levers(&self, front: f32, rear: f32) -> (f32, f32) {
        match self.release {
            true => (0.0, 0.0),
            false => (front, rear),
        }
    }
}

impl ReverseState {
    pub fn pushing(&self) -> bool {
        self.pushing
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// One step of `dt` seconds with the bike moving at `speed` m/s along itself (negative backwards).
    ///
    /// A request is a front lever above `PRESSED`, no throttle and a speed below `REQUEST_SPEED`; while it stands a timer runs
    /// and while it does not the timer is 0. Until the timer passes the hold time nothing changes. After it the force is the spec's
    /// times the lever times `1 + min(0, speed / max_speed)`: full at a standstill, 0 at the limit, and pushing back towards it
    /// beyond.
    pub fn step(&mut self, spec: &ReverseSpec, dt: f32, speed: f32, input: &BikeInput) -> ReverseDrive {
        let requested = spec.is_valid()
            && input.front_brake > PRESSED
            && input.throttle < PRESSED
            && speed < REQUEST_SPEED
            && dt.is_finite()
            && dt > 0.0;
        self.pushing = false;
        if !requested {
            self.timer = 0.0;
            return ReverseDrive::default();
        }
        self.timer += dt;
        if self.timer <= spec.hold_time {
            return ReverseDrive::default();
        }
        self.pushing = true;
        let taper = 1.0 + (speed / spec.max_speed).min(0.0);
        ReverseDrive { force: -spec.force * input.front_brake.clamp(0.0, 1.0) * taper, release: true }
    }
}

impl Bike {
    /// The reverse's part of one sub-step: the force on the body and the brakes let go. `speed` is the speed along the bike.
    pub(super) fn step_reverse(&mut self, dt: f32, speed: f32, input: &BikeInput) -> ReverseDrive {
        let Some(spec) = self.spec.reverse else { return ReverseDrive::default() };
        if self.fallen || self.departure.off {
            self.reverse.reset();
            return ReverseDrive::default();
        }
        let drive = self.reverse.step(&spec, dt, speed, input);
        if drive.force != 0.0 {
            let forward = self.body.rotation() * Vec3::Z;
            self.body.apply_force(forward * drive.force);
        }
        drive
    }

    /// Whether the rider is pushing the bike backwards.
    pub fn reversing(&self) -> bool {
        self.reverse.pushing()
    }
}

#[cfg(test)]
#[path = "reverse_tests.rs"]
mod tests;
