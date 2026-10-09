use super::{
    pc_emitter::PcSource,
    pc_particle::{PcParticle, weights},
};
use glam::{Quat, Vec3};
use nfsmw_data::vehicle_effects::PcEmitter;

fn profile() -> PcEmitter {
    PcEmitter {
        key: 1,
        texture: 2,
        grid: 0,
        fps: 0,
        random_frame: false,
        color: [[1.0; 4]; 4],
        size: [0.4; 4],
        keys: [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0],
        angles: [0.0; 4],
        angle_range: 0.0,
        rotation_variance: 0.0,
        random_direction: false,
        center: [0.0; 3],
        extent: [0.0; 3],
        velocity: [10.0, 0.0, 0.0],
        velocity_delta: [0.0; 3],
        acceleration: [0.0, 0.0, 2.0],
        acceleration_delta: [0.0; 3],
        one_sided: true,
        life: 0.5,
        life_variance: 0.0,
        rate: 200.0,
        rate_variance: 0.0,
        inherit: 0.0,
        inherit_variance: 0.0,
        speed: 0.0,
        speed_variance: 0.0,
        spread: 0.0,
        disc: false,
        drag: 0.0,
        gravity: 0.0,
        constraint: 0,
        alpha_kill: 0,
        no_alpha_kill: false,
        one_shot: false,
        delay: 0.0,
        random_delay: false,
        on: 0.3,
        on_variance: 0.0,
        off: 0.0,
        off_variance: 0.0,
        intensity: [0.0, 1.0],
    }
}

#[test]
fn pc_cubic_controls_use_authored_key_positions_and_can_overshoot() {
    let keys = [0.0, 0.1, 0.8, 1.0];
    for (index, age) in keys.into_iter().enumerate() {
        let w = weights(keys, age);
        for (i, value) in w.into_iter().enumerate() {
            assert_eq!(value, f32::from(u8::from(i == index)));
        }
    }
    let p = profile();
    let w = weights(p.keys, 0.5);
    let value = w.iter().zip([0.0, 1.0, 1.0, 0.0]).map(|(w, v)| w * v).sum::<f32>();
    assert!((value - 1.125).abs() < 0.0001, "a piecewise linear replacement would lose native cubic overshoot");
}

#[test]
fn pc_moves_with_drag_then_gravity_and_integer_life_ticks() {
    let mut p = profile();
    p.drag = 1.0;
    p.gravity = 3.0;
    let mut particle = PcParticle::spawn(p, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO, 1);
    particle.step(0.01);
    assert!((particle.point.x - 0.09).abs() < 0.00001);
    assert!((particle.point.z + 0.0003).abs() < 0.000001, "gravity overrides acceleration");
    assert!((particle.progress() - 10.0 / 512.0).abs() < 0.00001);
    particle.step(1.0);
    assert!(!particle.alive());
}

#[test]
fn pc_birth_variance_and_inheritance_keep_the_stock_ranges() {
    let mut p = profile();
    p.life_variance = 0.5;
    p.inherit = 0.8;
    p.inherit_variance = 0.5;
    for seed in 0..128 {
        let mut particle = PcParticle::spawn(p, Vec3::ZERO, Quat::IDENTITY, Vec3::Y * 10.0, seed);
        particle.step(0.01);
        assert!((0.04..=0.08).contains(&particle.point.y));
        assert!((0.02..=0.53).contains(&particle.progress()));
    }
}

#[test]
fn pc_animated_particle_keeps_the_authored_sprite_grid_and_wrap_quirk() {
    let mut p = profile();
    p.grid = 2;
    p.fps = 20;
    let mut particle = PcParticle::spawn(p, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO, 0);
    assert_eq!(particle.uv(), [[0.0, 0.0], [0.5, 0.0], [0.5, 0.5], [0.0, 0.5]]);
    particle.phase = 65000;
    particle.step(0.01);
    assert_eq!(particle.phase, 2741);
    assert_eq!(particle.uv()[0], [0.0, 0.0]);
}

fn source(p: PcEmitter, forced: bool) -> PcSource {
    PcSource::new(p, Vec3::ZERO, Quat::IDENTITY, Vec3::ZERO, 0.5, forced, 0)
}

#[test]
fn pc_rates_depend_on_intensity_and_preserve_fractional_and_cycle_boundaries() {
    let mut p = profile();
    p.rate = 100.0;
    p.on = 0.025;
    let mut s = source(p, true);
    assert_eq!(s.births(0.01), 0);
    assert_eq!(s.births(0.01), 0, "an accumulated value of exactly one waits");
    assert_eq!(s.births(0.01), 1);
    assert!(s.done());
    assert_eq!(s.births(0.5), 0);
    p.rate = 120.0;
    p.on = 0.015;
    let mut s = source(p, true);
    assert_eq!(s.births(0.01), 0);
    assert_eq!(s.births(0.01), 1, "the final dispatch includes native positive rollover");
}

#[test]
fn pc_delay_and_off_cycle_stop_emitting_without_birth_debt() {
    let mut p = profile();
    p.delay = 0.03;
    p.on = 0.02;
    p.off = 0.05;
    p.rate = 2000.0;
    let mut s = source(p, false);
    assert_eq!(s.births(0.01), 0);
    assert_eq!(s.births(0.01), 0);
    s.births(0.01);
    assert!(s.births(0.01) > 0);
    s.births(0.01);
    assert_eq!(s.births(0.01), 0);
    assert_eq!(s.births(0.01), 0);
}
