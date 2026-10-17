//! What the world gives the bars for one cop each step, and the health fraction it implies. Spec sections 3.2 and 13.2.

use glam::Vec3;

/// One chasing cop car this step. `key` is an id that stays the same for that car while it is sampled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub key: u32,
    /// Render space, metres: the car's position.
    pub position: Vec3,
    /// The car's damage so far (`Wreck::damage()`). It starts at 0 and only grows.
    pub damage: f32,
    /// True once the car is destroyed (`Wreck::is_destroyed()`). A flip can destroy a car whose health is above 0.
    pub destroyed: bool,
}

/// Health as a fraction of full: `1 - damage`, clamped to `0..=1`. 1 is unhurt.
pub fn health_fraction(damage: f32) -> f32 {
    (1.0 - damage).clamp(0.0, 1.0)
}

/// The health the bar shows: 0 once the car is destroyed, otherwise [`health_fraction`] of its damage.
pub fn displayed_health(sample: &Sample) -> f32 {
    match sample.destroyed {
        true => 0.0,
        false => health_fraction(sample.damage),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_bar::approx;

    fn sample(damage: f32, destroyed: bool) -> Sample {
        Sample { key: 1, position: Vec3::ZERO, damage, destroyed }
    }

    #[test]
    fn health_fraction_clamps_to_unit_range() {
        assert!(approx(health_fraction(0.0), 1.0));
        assert!(approx(health_fraction(0.4), 0.6));
        assert!(approx(health_fraction(1.2), 0.0));
        assert!(approx(health_fraction(-0.5), 1.0));
    }

    #[test]
    fn destroyed_car_shows_zero_health_even_with_damage_left() {
        assert!(approx(displayed_health(&sample(0.4, true)), 0.0));
        assert!(approx(displayed_health(&sample(0.4, false)), 0.6));
    }
}
