//! A roadblock that stands: its centre and direction, the counters the pursuit reads, the dodge test, the distance to the
//! target and the "perp cheating" timer. Spec: `docs/specs/ai-pursuit-roadblocks.md` section 5. The cars and props
//! belong to the caller; this holds the rules that read their positions.

use glam::Vec3;

/// A roadblock car within this distance (3-D) of the target counts the target as in sight (`kRoadBlockLOS`).
pub const SEES_YOU: f32 = 60.0;
/// A roadblock car within this distance of the target is engaged (`SetWithinEngagementRadius`).
pub const ENGAGE: f32 = 300.0;
/// Horizontal distance below which the cheat timer runs: the engagement radius plus 200 m.
pub const CHEAT_REACH: f32 = ENGAGE + 200.0;
/// The target stays within this distance of the roadblock's centre for the cheat timer to run (squared is 90 000).
pub const CHEAT_CENTRE: f32 = 300.0;
/// Seconds near the centre before the roadblock reports the target cheating.
pub const CHEAT_SECS: f32 = 20.0;
/// A car this much higher or lower than the target does not count for the horizontal distance (it is on another deck).
pub const SAME_LEVEL: f32 = 1.5;
/// What the distances read with no roadblock car to measure, metres.
pub const NO_CARS: f32 = 200.0;

/// One live roadblock car as the distance test sees it: whatever index the caller uses, and its position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Car {
    pub id: usize,
    pub position: Vec3,
}

/// The distances from a roadblock's live cars to the target (`GetMinDistanceToTarget`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// The smallest 3-D distance, and the car that has it. [`NO_CARS`] with none.
    pub min: f32,
    pub nearest: Option<usize>,
    /// The smallest horizontal distance among cars within [`SAME_LEVEL`] of the target's height. [`f32::MAX`] with none.
    pub min_xz: f32,
}

/// Measures `cars` against `target`.
pub fn reach(cars: impl IntoIterator<Item = Car>, target: Vec3) -> Reach {
    let mut out = Reach { min: f32::MAX, nearest: None, min_xz: f32::MAX };
    for car in cars {
        let distance = car.position.distance(target);
        if distance < out.min {
            out.min = distance;
            out.nearest = Some(car.id);
        }
        let flat = Vec3::new(car.position.x - target.x, 0.0, car.position.z - target.z).length();
        if (car.position.y - target.y).abs() < SAME_LEVEL && flat < out.min_xz {
            out.min_xz = flat;
        }
    }
    if out.nearest.is_none() {
        out.min = NO_CARS;
    }
    out
}

/// What a roadblock reports when the target gets past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dodged {
    /// The spike strips it had; they count as dodged too.
    pub spike_strips: u32,
}

/// The state of one roadblock (`AIRoadBlock`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Standing {
    pub centre: Vec3,
    /// The road's direction at the roadblock, the way the target was heading when it was built.
    pub dir: Vec3,
    pub dodged: bool,
    /// Cars of the roadblock that took damage, and wrecks. Either turns the follow formation off for good.
    pub damaged: u32,
    pub destroyed: u32,
    /// Spike strips among its props (sawhorses are not counted).
    pub spike_strips: u32,
    cheat_time: f32,
    cheating: bool,
}

impl Standing {
    pub fn new(centre: Vec3, dir: Vec3, spike_strips: u32) -> Self {
        Self { centre, dir, dodged: false, damaged: 0, destroyed: 0, spike_strips, cheat_time: 0.0, cheating: false }
    }

    /// The target has passed the centre: the centre is behind it along the road's direction. Reports it once.
    pub fn dodge(&mut self, target: Vec3) -> Option<Dodged> {
        if self.dodged {
            return None;
        }
        let to_centre = (self.centre - target).normalize_or_zero();
        if to_centre.dot(self.dir) >= 0.0 {
            return None;
        }
        self.dodged = true;
        Some(Dodged { spike_strips: self.spike_strips })
    }

    /// The cheat timer (`GetMinDistanceToTarget` with a time step): while a car is within [`CHEAT_REACH`] and the target
    /// stays within [`CHEAT_CENTRE`] of the centre the timer runs; it reports at [`CHEAT_SECS`], or at once in the
    /// cool-down. Away from the centre it resets. With no car near, it holds.
    pub fn watch(&mut self, dt: f32, min_xz: f32, target: Vec3, cooling_down: bool) {
        if dt <= 0.0 {
            return;
        }
        self.cheating = false;
        if min_xz >= CHEAT_REACH {
            return;
        }
        if target.distance_squared(self.centre) >= CHEAT_CENTRE * CHEAT_CENTRE {
            self.cheat_time = 0.0;
            return;
        }
        self.cheat_time += dt;
        self.cheating = self.cheat_time > CHEAT_SECS || cooling_down;
    }

    /// The roadblock reports the target cheating.
    pub fn is_cheating(&self) -> bool {
        self.cheating
    }

    /// The roadblock is up, not passed and untouched: the pursuit holds the follow formation (`AIPursuit` formation pick).
    pub fn forces_follow(&self) -> bool {
        !self.dodged && self.damaged == 0 && self.destroyed == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car(id: usize, x: f32, y: f32, z: f32) -> Car {
        Car { id, position: Vec3::new(x, y, z) }
    }

    #[test]
    fn the_nearest_car_and_the_flat_distance_are_measured() {
        let r = reach([car(0, 100.0, 0.0, 0.0), car(1, 40.0, 0.5, 30.0), car(2, 60.0, 9.0, 0.0)], Vec3::ZERO);
        assert_eq!(r.nearest, Some(1));
        assert!((r.min - Vec3::new(40.0, 0.5, 30.0).length()).abs() < 1e-4);
        assert!((r.min_xz - 50.0).abs() < 1e-4, "the car 9 m up is on another deck and is skipped");
    }

    #[test]
    fn no_cars_read_two_hundred() {
        let r = reach(std::iter::empty(), Vec3::ZERO);
        assert_eq!((r.min, r.nearest), (NO_CARS, None));
        assert_eq!(r.min_xz, f32::MAX);
    }

    #[test]
    fn passing_the_centre_is_a_dodge_reported_once() {
        let mut rb = Standing::new(Vec3::new(250.0, 0.0, 0.0), Vec3::X, 2);
        assert_eq!(rb.dodge(Vec3::ZERO), None, "the centre is ahead");
        assert_eq!(rb.dodge(Vec3::new(251.0, 0.0, 0.0)), Some(Dodged { spike_strips: 2 }));
        assert_eq!(rb.dodge(Vec3::new(400.0, 0.0, 0.0)), None, "a dodged roadblock is not counted again");
        assert!(rb.dodged);
    }

    #[test]
    fn the_cheat_timer_runs_near_the_centre_and_fires_after_twenty_seconds() {
        let mut rb = Standing::new(Vec3::ZERO, Vec3::X, 0);
        let target = Vec3::new(-200.0, 0.0, 0.0);
        for _ in 0..19 {
            rb.watch(1.0, 200.0, target, false);
        }
        assert!(!rb.is_cheating());
        rb.watch(1.0, 200.0, target, false);
        assert!(!rb.is_cheating(), "twenty seconds is not over twenty");
        rb.watch(1.0, 200.0, target, false);
        assert!(rb.is_cheating());
    }

    #[test]
    fn the_cool_down_reports_cheating_at_once() {
        let mut rb = Standing::new(Vec3::ZERO, Vec3::X, 0);
        rb.watch(0.5, 100.0, Vec3::new(-100.0, 0.0, 0.0), true);
        assert!(rb.is_cheating());
    }

    #[test]
    fn leaving_the_centre_resets_the_timer_and_a_far_car_holds_it() {
        let mut rb = Standing::new(Vec3::ZERO, Vec3::X, 0);
        for _ in 0..10 {
            rb.watch(1.0, 100.0, Vec3::new(-100.0, 0.0, 0.0), false);
        }
        rb.watch(1.0, 600.0, Vec3::new(-100.0, 0.0, 0.0), false);
        for _ in 0..11 {
            rb.watch(1.0, 100.0, Vec3::new(-100.0, 0.0, 0.0), false);
        }
        assert!(rb.is_cheating(), "the far reading held the 10 s: 10 + 11 is over 20");
        rb.watch(1.0, 100.0, Vec3::new(-400.0, 0.0, 0.0), false);
        assert!(!rb.is_cheating());
        for _ in 0..19 {
            rb.watch(1.0, 100.0, Vec3::new(-100.0, 0.0, 0.0), false);
        }
        assert!(!rb.is_cheating(), "the walk away reset it");
    }

    #[test]
    fn follow_is_forced_only_while_it_is_up_whole_and_unpassed() {
        let mut rb = Standing::new(Vec3::ZERO, Vec3::X, 0);
        assert!(rb.forces_follow());
        rb.damaged = 1;
        assert!(!rb.forces_follow());
        rb.damaged = 0;
        rb.dodged = true;
        assert!(!rb.forces_follow());
    }
}
