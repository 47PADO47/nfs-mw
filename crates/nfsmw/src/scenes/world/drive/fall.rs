//! Noticing that the car has fallen off the map.

/// A car with no wheel down and nothing within this many metres below it is falling.
pub const PROBE: f32 = 30.0;
/// ...for this many physics steps in a row has fallen off the map.
const STEPS: u32 = 45;
/// ...or it is this far under the last road it stood on (metres).
const BELOW_ROAD: f32 = 80.0;

#[derive(Debug, Default)]
pub struct FallWatch {
    falling_steps: u32,
}

impl FallWatch {
    pub fn reset(&mut self) {
        self.falling_steps = 0;
    }

    /// Account for `steps` physics steps. `nothing_below` is only asked for when no wheel is down.
    /// `last_road_height` is the height of the last road the car stood on. True once the car has
    /// fallen off the map.
    pub fn update(
        &mut self,
        wheels_down: usize,
        steps: u32,
        height: f32,
        last_road_height: Option<f32>,
        nothing_below: impl FnOnce() -> bool,
    ) -> bool {
        if wheels_down > 0 {
            self.falling_steps = 0;
        } else if steps > 0 {
            self.falling_steps = if nothing_below() { self.falling_steps + steps } else { 0 };
        }
        let under_the_road = last_road_height.is_some_and(|h| height < h - BELOW_ROAD);
        let fallen = self.falling_steps >= STEPS || under_the_road;
        if fallen {
            self.falling_steps = 0;
        }
        fallen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_car_in_the_air_over_the_road_has_not_fallen() {
        let mut w = FallWatch::default();
        // A jump: no wheel down for two seconds, but the road is just below.
        assert!((0..120).all(|_| !w.update(0, 1, 150.0, Some(150.0), || false)));
    }

    #[test]
    fn nothing_below_for_three_quarters_of_a_second_is_a_fall() {
        let mut w = FallWatch::default();
        let fell_at = (1..200).find(|_| w.update(0, 1, 140.0, Some(150.0), || true));
        assert_eq!(fell_at, Some(STEPS as usize));
        // And it starts counting again afterwards.
        assert!(!w.update(0, 1, 140.0, Some(150.0), || true));
    }

    #[test]
    fn landing_resets_the_count() {
        let mut w = FallWatch::default();
        for _ in 0..40 {
            assert!(!w.update(0, 1, 140.0, None, || true));
        }
        assert!(!w.update(2, 1, 140.0, None, || true));
        assert!((0..40).all(|_| !w.update(0, 1, 140.0, None, || true)), "a wheel down starts the count over");
    }

    #[test]
    fn far_under_the_last_road_is_a_fall_at_once() {
        let mut w = FallWatch::default();
        assert!(!w.update(4, 1, 100.0, Some(150.0), || true));
        assert!(w.update(4, 1, 60.0, Some(150.0), || true));
        // Without a road to compare with, only the falling count decides.
        assert!(!FallWatch::default().update(4, 1, -500.0, None, || true));
    }

    #[test]
    fn the_ground_probe_is_only_asked_when_it_matters() {
        let mut w = FallWatch::default();
        w.update(4, 1, 150.0, None, || panic!("a wheel is down"));
        w.update(0, 0, 150.0, None, || panic!("no step ran"));
    }
}
