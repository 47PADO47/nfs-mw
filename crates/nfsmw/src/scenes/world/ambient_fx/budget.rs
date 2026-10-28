//! Reserve the authored live population without enlarging every ambient emitter.
use blackbox_particles::{EmitterSpec, MAX_PARTICLES};

const BASE: usize = 128;

pub(super) fn capacity(spec: &EmitterSpec) -> usize {
    if [spec.rate, spec.rate_variance, spec.life, spec.life_variance, spec.on_cycle, spec.on_cycle_variance]
        .iter()
        .any(|value| !value.is_finite())
    {
        return BASE;
    }
    let rate = (f64::from(spec.rate) * (1.0 - f64::from(spec.rate_variance))).max(0.0);
    // Emitter::birth clamps individual life to 60 seconds. Negative variance can increase it.
    let life = f64::from(spec.life).max(f64::from(spec.life) * (1.0 - f64::from(spec.life_variance))).clamp(0.0, 60.0);
    let mut window = life;
    if spec.one_shot && (spec.on_cycle != 0.0 || spec.on_cycle_variance != 0.0) {
        let on = (f64::from(spec.on_cycle) + f64::from(spec.on_cycle_variance).max(0.0)).max(0.0);
        window = window.min(on);
    }
    // With no on-cycle, the engine uses the particle-life window for an authored burst.
    // Two spare particles cover the spawn-before-age boundary without increasing ordinary small effects.
    ((rate * window).ceil() as usize).saturating_add(2).clamp(BASE, MAX_PARTICLES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserves_steam_population_but_not_long_lived_short_bursts() {
        let steam = EmitterSpec { rate: 100.0, life: 2.0, ..Default::default() };
        assert_eq!(capacity(&steam), 202);
        let birds = EmitterSpec { rate: 50.0, life: 10.0, on_cycle: 1.0, one_shot: true, ..Default::default() };
        assert_eq!(capacity(&birds), BASE);
        let leaves = EmitterSpec { rate: 20.0, life: 10.0, on_cycle: 6.0, one_shot: true, ..Default::default() };
        assert_eq!(capacity(&leaves), BASE);
        assert_eq!(capacity(&EmitterSpec { rate_variance: 0.5, ..steam }), BASE);
    }

    #[test]
    fn invalid_and_extreme_parameters_remain_bounded() {
        assert_eq!(capacity(&EmitterSpec { rate: f32::NAN, ..Default::default() }), BASE);
        assert_eq!(capacity(&EmitterSpec { life: f32::INFINITY, ..Default::default() }), BASE);
        assert_eq!(capacity(&EmitterSpec { rate: -100.0, ..Default::default() }), BASE);
        assert_eq!(capacity(&EmitterSpec { rate: f32::MAX, life: f32::MAX, ..Default::default() }), MAX_PARTICLES);
        assert_eq!(capacity(&EmitterSpec { rate: 100.0, life: 1.0, life_variance: -1.0, ..Default::default() }), 202);
    }
}
