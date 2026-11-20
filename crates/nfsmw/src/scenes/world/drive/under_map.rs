//! Noticing that a bike or its thrown rider has gone under the map (`docs/specs/ride6-feel-round14.md`, section 4).
//!
//! The world has ground only where the roads are: a body that leaves it, through the edge of a road or through a surface that a contact
//! pushed it under, falls for ever, and the camera that follows it falls with it. [`fall.rs`](super::fall) catches a car that has no
//! wheel down and nothing within 30 m below for three quarters of a second; this is for what that misses or catches late: a rider
//! (he has no wheels), a bike that lies sunk in a surface, a state that is not a number, a speed no bike has.
//!
//! A body is under the map when nothing is below it (no ground within [`BELOW_M`]) and
//!
//! - it is a **rider**: he has no business without ground under him, however high he is (he is thrown across the edge of a road, into the
//!   void behind it); or
//! - it is a **bike** and either a surface is just above it ([`COVER_M`]: it is beneath the road) or it is lower than the last place that
//!   held it up by [`SINK_M`] (a bike may jump a gap that has nothing below it).
//!
//! That holds for [`HOLD_S`] before it counts, so a glitch of one step is not a reset. A body with ground below it is falling onto
//! something, however far (a rider thrown off a bridge); the old watch's 80 m below the last road still catches a bike's fall that never
//! finds ground.

/// How far below a body ground is looked for (m): the same reach as the old watch's.
pub const BELOW_M: f32 = 30.0;
/// How far above a body a surface counts as covering it (m).
pub const COVER_M: f32 = 6.0;
/// How far under the last place that held the body up it must be (m) when nothing covers it.
pub const SINK_M: f32 = 0.5;
/// How long it must stay under before it counts (s).
pub const HOLD_S: f32 = 0.1;
/// A speed (m/s) no bike or rider has: a bike's top speed is 90 and a ragdoll is held to 100 (`blackbox_vehicle::ragdoll`).
pub const MAX_SPEED: f32 = 150.0;

/// What the host sees around a body this step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Probe {
    /// The body's height (render space, m) and its speed (m/s).
    pub height: f32,
    pub speed: f32,
    /// The height of the last place that held the bike or its rider up, if there was one.
    pub floor: Option<f32>,
    /// The body is a thrown rider, not a bike.
    pub rider: bool,
    /// Ground within [`BELOW_M`] below the body, and a surface within [`COVER_M`] above it.
    pub ground_below: bool,
    pub covered: bool,
    /// Every number of the body's state is finite.
    pub finite: bool,
}

/// The time a body has been under the map.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct UnderMap {
    seconds: f32,
}

impl UnderMap {
    /// Forget what was seen (a placement).
    pub fn clear(&mut self) {
        self.seconds = 0.0;
    }

    /// Whether the probe is under the map by itself, with no hold.
    pub fn is_under(probe: &Probe) -> bool {
        if probe.ground_below {
            return false;
        }
        probe.rider || probe.covered || probe.floor.is_some_and(|floor| probe.height < floor - SINK_M)
    }

    /// One step of `dt` seconds. True when the body is under the map and must be put back: a state that is not a number or a speed
    /// that no body has counts at once, the rest after [`HOLD_S`].
    pub fn update(&mut self, probe: &Probe, dt: f32) -> bool {
        let wild = !probe.finite || !probe.height.is_finite() || probe.speed.is_nan() || probe.speed > MAX_SPEED;
        if wild {
            self.seconds = 0.0;
            return true;
        }
        if !Self::is_under(probe) {
            self.seconds = 0.0;
            return false;
        }
        if !(dt > 0.0 && dt.is_finite()) {
            return false;
        }
        self.seconds += dt;
        if self.seconds < HOLD_S {
            return false;
        }
        self.seconds = 0.0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn riding() -> Probe {
        Probe {
            height: 214.5,
            speed: 30.0,
            floor: Some(214.5),
            rider: false,
            ground_below: true,
            covered: false,
            finite: true,
        }
    }

    fn sunk() -> Probe {
        Probe { height: 210.0, ground_below: false, ..riding() }
    }

    /// How many steps of `probe` it takes to count, if ever within two seconds.
    fn steps_to_count(probe: Probe) -> Option<u32> {
        let mut watch = UnderMap::default();
        (1..=120).find(|_| watch.update(&probe, DT))
    }

    #[test]
    fn a_body_riding_on_the_road_is_never_under_the_map() {
        assert_eq!(steps_to_count(riding()), None);
    }

    #[test]
    fn a_body_with_nothing_below_and_lower_than_the_last_floor_counts_within_a_third_of_a_second() {
        let steps = steps_to_count(sunk()).expect("under the map");
        assert!(steps as f32 * DT <= 0.3, "{steps} steps");
        assert!(steps >= 2, "not on the first glimpse: {steps}");
    }

    #[test]
    fn a_body_with_nothing_below_but_not_lower_than_the_floor_is_only_in_the_air() {
        // Over a gap or a canyon: the old watch's 80 m and its quarter of a second handle that, not this one.
        let jumping = Probe { height: 215.0, ground_below: false, ..riding() };
        assert_eq!(steps_to_count(jumping), None);
        let barely = Probe { height: 214.5 - SINK_M + 0.05, ground_below: false, ..riding() };
        assert_eq!(steps_to_count(barely), None);
    }

    #[test]
    fn a_body_falling_onto_a_road_far_below_has_ground_below_and_is_not_lost() {
        // A rider thrown off a bridge: 6 m under the road he was on, with the road below him.
        let thrown = Probe { height: 208.0, ground_below: true, ..riding() };
        assert_eq!(steps_to_count(thrown), None);
    }

    #[test]
    fn a_body_under_a_surface_with_nothing_below_counts_even_with_no_floor_known() {
        let beneath = Probe { floor: None, covered: true, ground_below: false, ..riding() };
        assert!(steps_to_count(beneath).is_some());
        let lost = Probe { floor: None, covered: false, ground_below: false, ..riding() };
        assert_eq!(steps_to_count(lost), None, "nothing to compare with and nothing over it: the old watch's job");
    }

    #[test]
    fn a_rider_with_nothing_below_him_is_lost_wherever_he_is() {
        // Thrown across the edge of the road at the height of the road: no floor to be under, and still lost.
        let thrown = Probe { height: 214.8, rider: true, ground_below: false, ..riding() };
        let steps = steps_to_count(thrown).expect("lost");
        assert!(steps as f32 * DT <= 0.3, "{steps} steps");
        assert_eq!(
            steps_to_count(Probe { ground_below: true, ..thrown }),
            None,
            "with ground below he is falling onto it"
        );
    }

    #[test]
    fn a_glitch_of_one_step_is_not_a_reset() {
        let mut watch = UnderMap::default();
        assert!(!watch.update(&sunk(), DT));
        assert!(!watch.update(&riding(), DT));
        for _ in 0..3 {
            assert!(!watch.update(&sunk(), DT), "the count started again");
        }
    }

    #[test]
    fn a_state_that_is_not_a_number_or_a_speed_nobody_has_counts_at_once() {
        let mut watch = UnderMap::default();
        assert!(watch.update(&Probe { finite: false, ..riding() }, DT));
        assert!(watch.update(&Probe { height: f32::NAN, ..riding() }, DT));
        assert!(watch.update(&Probe { speed: 400.0, ..riding() }, DT));
        assert!(watch.update(&Probe { speed: f32::NAN, ..riding() }, DT));
        assert!(!watch.update(&Probe { speed: 90.0, ..riding() }, DT), "a bike at its top speed is fine");
    }

    #[test]
    fn it_counts_again_after_a_reset_and_a_placement_clears_it() {
        let mut watch = UnderMap::default();
        let first = (1..=120).find(|_| watch.update(&sunk(), DT)).expect("counted");
        let second = (1..=120).find(|_| watch.update(&sunk(), DT)).expect("counted again");
        assert_eq!(first, second);
        watch.update(&sunk(), DT);
        watch.clear();
        assert_eq!((1..=120).find(|_| watch.update(&sunk(), DT)), Some(first));
    }
}
