//! A bounded simulation clock for traffic placement, independent of render frame rate.
//! Original registration runs about every two rendered frames; this uses the nominal 60 FPS cadence.

pub(super) const PERIOD: f32 = 1.0 / 30.0;

/// At most one activation per physics step. Disabled traffic cannot accumulate a catch-up burst.
pub(super) fn due(clock: &mut f32, dt: f32, enabled: bool) -> bool {
    if !enabled {
        *clock = 0.0;
        return false;
    }
    *clock += dt;
    if *clock < PERIOD {
        return false;
    }
    *clock = (*clock - PERIOD).min(PERIOD);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_follows_simulated_time_instead_of_render_frames() {
        for physics_steps_per_frame in [1, 2, 4] {
            let mut clock = 0.0;
            let mut activations = 0;
            for _ in 0..(600 / physics_steps_per_frame) {
                for _ in 0..physics_steps_per_frame {
                    activations += usize::from(due(&mut clock, 1.0 / 60.0, true));
                }
            }
            assert_eq!(activations, 300, "ten simulated seconds, batched independently of rendering");
        }
    }

    #[test]
    fn disabled_population_does_not_bank_activation_time() {
        let mut clock = PERIOD;
        for _ in 0..600 {
            assert!(!due(&mut clock, 1.0 / 60.0, false));
        }
        assert!(!due(&mut clock, 1.0 / 60.0, true));
        assert!(due(&mut clock, 1.0 / 60.0, true));
    }
}
