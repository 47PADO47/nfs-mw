//! The car's reset trail, its flip timer and its post-reset blink (`docs/specs/vehicle-reset.md`). The car remembers the
//! last four places it drove through with every wheel down, at least 2 m apart; a reset goes back to the oldest, about
//! 6 to 8 m behind, so the car comes back on its own path rather than on the road nearest where it ended up. After a reset
//! the place it went to is the trail's only entry, so another reset goes there again until four new places are recorded.

use glam::Vec3;

/// The places the trail keeps.
const LENGTH: usize = 4;
/// The least distance from the newest place to record another, metres.
const SPACING: f32 = 2.0;
/// Seconds the car must stay on its roof or side before it is put back.
const FLIPPED_SECS: f32 = 4.0;
/// The tilt past which the car counts as flipped: its up axis more than 60 degrees from world up.
const FLIPPED_UP: f32 = 0.5;
/// The blink's hidden and whole periods, seconds: hidden 0.05 s, then shown 0.07 s.
const BLINK_HIDDEN: f32 = 0.05;
const BLINK_PERIOD: f32 = 0.12;

/// One remembered place: the ground under the car (map x, y and the ground's height) and the heading it drove at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crumb {
    pub position: Vec3,
    pub heading: f32,
}

/// The last places, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Trail {
    crumbs: Vec<Crumb>,
}

impl Trail {
    /// Records `crumb` when the trail is empty or the car is at least 2 m from the newest place. The caller only offers a
    /// place while every wheel is on the ground over the world. Returns whether it was recorded.
    pub fn record(&mut self, crumb: Crumb) -> bool {
        if self.crumbs.last().is_some_and(|newest| newest.position.distance(crumb.position) < SPACING) {
            return false;
        }
        if self.crumbs.len() == LENGTH {
            self.crumbs.remove(0);
        }
        self.crumbs.push(crumb);
        true
    }

    /// Where a reset goes: the oldest place; `None` with no place recorded (the reset is refused).
    pub fn oldest(&self) -> Option<Crumb> {
        self.crumbs.first().copied()
    }

    /// A reset put the car at `used`: it is the trail's only place.
    pub fn reset_to(&mut self, used: Crumb) {
        self.crumbs.clear();
        self.crumbs.push(used);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.crumbs.len()
    }
}

/// How long the car has been on its roof or its side.
#[derive(Debug, Clone, Copy, Default)]
pub struct FlipTimer {
    secs: f32,
}

impl FlipTimer {
    /// One step: `flipped` adds `dt`, anything else starts it again. True when the car has stayed flipped over 4 s.
    pub fn update(&mut self, flipped: bool, dt: f32) -> bool {
        self.secs = match flipped {
            true => self.secs + dt,
            false => 0.0,
        };
        self.secs > FLIPPED_SECS
    }

    pub fn clear(&mut self) {
        self.secs = 0.0;
    }
}

/// Whether the car is flipped: not every wheel is on the ground and its up axis (`up_z`, the world-up component) is more
/// than 60 degrees from world up. The original also asks for 60 degrees from the ground's normal; on the flat ground of a
/// road the two agree.
pub fn flipped(wheels_down: usize, up_z: f32) -> bool {
    wheels_down < 4 && up_z < FLIPPED_UP
}

/// The car's reset state: its trail, its flip timer, its blink, the place a reset asked for and the manual resets the
/// scene has not taken yet.
#[derive(Debug, Clone, Default)]
pub struct ResetState {
    pub trail: Trail,
    pub flip: FlipTimer,
    pub blink: Blink,
    /// The place a reset asked for, until the car is put there or the reset fails.
    pub pending: Option<Crumb>,
    /// Manual resets (the reset key) since the scene last took them; the police start their reset penalty on them.
    pub manual: u32,
}

/// The blink while a reset's grace runs: hidden 0.05 s, shown 0.07 s.
#[derive(Debug, Clone, Copy, Default)]
pub struct Blink {
    phase: f32,
    on: bool,
}

impl Blink {
    /// One frame of `dt` seconds while `on` (a reset's grace runs).
    pub fn update(&mut self, on: bool, dt: f32) {
        self.on = on;
        if !on {
            return;
        }
        self.phase += dt;
        if self.phase > BLINK_HIDDEN {
            self.phase -= BLINK_PERIOD;
        }
    }

    /// Whether the car is hidden this frame.
    pub fn hidden(&self) -> bool {
        self.on && self.phase > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(y: f32) -> Crumb {
        Crumb { position: Vec3::new(0.0, y, 0.0), heading: 0.0 }
    }

    #[test]
    fn the_trail_keeps_four_places_two_metres_apart_and_a_reset_goes_to_the_oldest() {
        let mut trail = Trail::default();
        assert!(trail.oldest().is_none(), "no place yet: a reset is refused");
        assert!(trail.record(at(0.0)));
        assert!(!trail.record(at(1.5)), "too close to the newest");
        for y in [2.0, 4.0, 6.0, 8.0] {
            assert!(trail.record(at(y)));
        }
        assert_eq!(trail.len(), 4);
        assert_eq!(trail.oldest(), Some(at(2.0)), "6 m behind the car at 8 m");
        trail.reset_to(at(2.5));
        assert_eq!(trail.oldest(), Some(at(2.5)));
        assert!(trail.record(at(4.5)));
        assert_eq!(trail.oldest(), Some(at(2.5)), "another reset goes to the same place until four new ones are in");
    }

    #[test]
    fn the_car_is_put_back_after_more_than_four_seconds_on_its_roof() {
        let mut timer = FlipTimer::default();
        assert!(flipped(2, 0.2) && !flipped(4, 0.2) && !flipped(0, 0.9));
        for _ in 0..40 {
            assert!(!timer.update(true, 0.1));
        }
        assert!(timer.update(true, 0.1), "past 4 s");
        timer.update(false, 0.1);
        assert!(!timer.update(true, 0.1), "any frame back on the wheels starts again");
    }

    #[test]
    fn the_blink_hides_the_car_for_five_hundredths_in_every_twelve() {
        let mut blink = Blink::default();
        let mut hidden = 0;
        for _ in 0..1200 {
            blink.update(true, 0.001);
            hidden += usize::from(blink.hidden());
        }
        assert!((hidden as f32 / 1200.0 - 0.05 / 0.12).abs() < 0.02, "{hidden}");
        blink.update(false, 0.001);
        assert!(!blink.hidden(), "shown once the grace is over");
    }
}
