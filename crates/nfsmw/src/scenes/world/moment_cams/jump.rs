//! The jump camera's look ahead at a takeoff: the path the car would fly with no forces but its weight and downforce,
//! against the ground under it. Spec: `docs/specs/game-moment-camera.md` section 2.2. Render space (z up).

use glam::Vec3;

use super::rules::{Flight, JUMP_SPEED};

/// How far ahead the path is followed (s).
pub const HORIZON: f32 = 3.0;
/// Ground steeper than this (the normal's up component) is not ground to land on: the sample is skipped.
const LANDABLE: f32 = 0.5;

/// What the prediction asks of the world.
pub trait Ground {
    /// The ground under `at`: its height and the up component of its normal.
    fn under(&self, at: Vec3) -> Option<(f32, f32)>;
    /// Whether the straight line from `a` to `b` is clear of the world's geometry.
    fn clear(&self, a: Vec3, b: Vec3) -> bool;
}

/// The flight from `start` (the tyres' mean contact point) at `velocity`, with `gravity` the vertical acceleration (the
/// body's gravity less its downforce per mass, negative). `None` below [`JUMP_SPEED`] or when the way ahead is blocked.
///
/// The path is sampled once per metre per second of speed over [`HORIZON`]: `n` steps of `HORIZON / n`. The first
/// sample is the start's own height (clamped at zero); a car already off the ground counts its samples a step later.
/// The landing is the first sample at or under the ground; the outputs are the mean and the peak of the heights up to
/// it and the time of that sample (the last sample's time when the path never lands within the horizon).
pub fn predict(start: Vec3, velocity: Vec3, gravity: f32, ground: &impl Ground) -> Option<Flight> {
    let speed = velocity.length();
    if speed < JUMP_SPEED {
        return None;
    }
    let steps = (speed * HORIZON / 3.0) as i32;
    if steps < 1 {
        return None;
    }
    let dt = HORIZON / steps as f32;
    if !ground.clear(start, start + velocity * 0.5) {
        return None;
    }
    let height_at = |at: Vec3| ground.under(at).map(|(height, _)| at.z - height);
    let first = height_at(start).unwrap_or(0.0).max(0.0);
    let offset = match first > 0.0 {
        true => dt,
        false => 0.0,
    };
    let mut sum = first;
    let mut peak = first;
    let mut time = 0.0;
    let mut i = 1;
    while i < steps {
        time = i as f32 * dt + offset;
        let at = start + velocity * time + Vec3::new(0.0, 0.0, 0.5 * gravity * time * time);
        if let Some((height, _)) = ground.under(at).filter(|(_, up)| *up >= LANDABLE) {
            let above = at.z - height;
            if above <= 0.0 {
                break;
            }
            sum += above;
            peak = peak.max(above);
        }
        i += 1;
    }
    Some(Flight { mean_height: sum / i as f32, peak, airtime: time })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat ground at height 0 with a drop to -10 beyond `edge` (x).
    struct Cliff {
        edge: f32,
    }

    impl Ground for Cliff {
        fn under(&self, at: Vec3) -> Option<(f32, f32)> {
            Some((if at.x > self.edge { -10.0 } else { 0.0 }, 1.0))
        }
        fn clear(&self, _: Vec3, _: Vec3) -> bool {
            true
        }
    }

    #[test]
    fn a_ramp_launch_flies_a_parabola_and_lands() {
        // 30 m/s with 5 m/s up over flat ground: in the air for about 1 s, about 1.3 m high.
        let flight = predict(Vec3::ZERO, Vec3::new(30.0, 0.0, 5.0), -9.81, &Cliff { edge: 1000.0 }).unwrap();
        assert!((0.9..=1.15).contains(&flight.airtime), "landing quantised to the 0.1 s step: {flight:?}");
        assert!((1.1..1.35).contains(&flight.peak), "{flight:?}");
        assert!(flight.mean_height > 0.5 && flight.mean_height < flight.peak);
    }

    #[test]
    fn driving_off_a_drop_is_a_big_flight() {
        let flight = predict(Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0), -9.81, &Cliff { edge: 2.0 }).unwrap();
        assert!(flight.peak > 5.0, "{flight:?}");
        assert!(flight.airtime > 1.0 && flight.airtime < 1.6, "lands about 1.4 s later: {flight:?}");
        assert!(super::super::rules::jump_worth_a_shot(flight, 30.0, false));
    }

    #[test]
    fn slow_cars_and_blocked_ways_predict_nothing() {
        assert!(predict(Vec3::ZERO, Vec3::new(9.0, 0.0, 0.0), -9.81, &Cliff { edge: 2.0 }).is_none());
        struct Wall;
        impl Ground for Wall {
            fn under(&self, _: Vec3) -> Option<(f32, f32)> {
                Some((0.0, 1.0))
            }
            fn clear(&self, _: Vec3, _: Vec3) -> bool {
                false
            }
        }
        assert!(predict(Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0), -9.81, &Wall).is_none());
    }
}
