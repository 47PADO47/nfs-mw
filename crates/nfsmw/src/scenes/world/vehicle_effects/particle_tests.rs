use super::*;

fn spark() -> EmitterStyle {
    EmitterStyle {
        color: [1.0, 0.86, 0.75, 1.0],
        life: 1.0,
        life_variance: 0.05,
        count: 30.0,
        extent: [0.0, 0.5, 0.0],
        velocity: [0.0, 0.0, 1.0],
        variation: [0.4, 0.4, 2.0],
        inherit: [-1.0, -1.0, -0.1],
        gravity: -12.0,
        gravity_delta: 9.0,
        length: 255.0,
        length_delta: 140.0,
        height: 62.0,
        ..EmitterStyle::default()
    }
}

#[test]
fn controlled_uniform_inputs_match_x86_spawn_measurements() {
    for (u, x, z, gravity, y) in
        [(0.1, -40.8, -0.6, -19.2, -0.2), (0.5, -60.0, 1.0, -12.0, 0.0), (0.9, -79.2, 2.6, -4.8, 0.2)]
    {
        let p = Particle::sample(spark(), Vec3::ZERO, Quat::IDENTITY, Vec3::X * 60.0, |_| u, 1.0);
        assert!(p.velocity.abs_diff_eq(Vec3::new(x, 0.0, z), 1e-4));
        assert!((p.gravity - gravity).abs() < 1e-4);
        assert!(p.origin.abs_diff_eq(Vec3::new(0.0, y, 0.0), 1e-5));
        assert!((p.life - 0.95).abs() < 1e-6);
        assert_eq!(p.duration, 255.0 / 2048.0);
        assert_eq!(p.width, 62.0 / 2048.0);
        assert_eq!(p.color[3], 1.0);
    }
    assert_eq!(count(spark(), 0.1), 30);
    assert_eq!(count(spark(), 1.0), 30);
}

#[test]
fn wind_volume_alpha_and_direction_match_emulated_reference() {
    let trail = EmitterStyle {
        color: [0.64, 1.0, 0.85, 0.13],
        life: 0.25,
        life_variance: 0.25,
        count: 15.0,
        extent: [1.5, 2.0, 0.75],
        velocity: [0.0, 0.0, -1.0],
        inherit: [-0.3, -0.3, -0.1],
        gravity: -2.0,
        length: 200.0,
        length_delta: 50.0,
        height: 255.0,
        ..EmitterStyle::default()
    };
    let p = Particle::sample(trail, Vec3::ZERO, Quat::IDENTITY, Vec3::X * 60.0, |_| 0.1, 0.1);
    assert!(p.origin.abs_diff_eq(Vec3::new(-0.6, -0.8, -0.3), 1e-5));
    assert_eq!(p.velocity, Vec3::new(-18.0, 0.0, -1.0));
    assert_eq!(p.life, 0.1875);
    assert_eq!(p.color[3], 4.0 / 255.0);
    assert_eq!(p.duration, 205.0 / 2048.0);
    assert_eq!(count(trail, 0.1), count(trail, 0.75));
    let rotated = Particle::sample(
        trail,
        Vec3::new(3.0, 4.0, 5.0),
        Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        Vec3::Y * 60.0,
        |_| 0.1,
        0.1,
    );
    assert!(
        rotated.origin.abs_diff_eq(Vec3::new(3.8, 3.4, 4.7), 1e-5),
        "full body-relative volume rotates with the car"
    );
    assert!(rotated.velocity.abs_diff_eq(Vec3::new(0.0, -18.0, -1.0), 1e-5));
}

#[test]
fn parabola_and_swept_bounce_preserve_remaining_lifetime_and_substep() {
    let mut p = Particle::sample(spark(), Vec3::Z, Quat::IDENTITY, Vec3::ZERO, |_| 0.5, 1.0);
    p.velocity = -Vec3::Z * 10.0;
    p.gravity = -2.0;
    p.elasticity = 160.0 / 255.0;
    p.age(0.2);
    assert!((p.position(0.2).z + 1.08).abs() < 1e-5);
    p.bounce(0.5, Vec3::ZERO, Vec3::Z, 0.2);
    assert!((p.velocity.z - 10.4 * 160.0 / 255.0).abs() < 1e-5);
    assert!((p.life - 0.85).abs() < 1e-5);
    assert_eq!(p.age, 0.1);
    assert!(p.position(p.age).z > 0.0);
    assert_eq!(p.bounces, 1);
}

#[test]
fn independent_uniform_samples_are_deterministic_and_cover_the_full_volume() {
    let values: Vec<_> = (0..1000).map(|seed| uniform(seed, 0)).collect();
    assert!(values.iter().all(|v| (0.0..1.0).contains(v)));
    assert!(values.iter().any(|v| *v < 0.01) && values.iter().any(|v| *v > 0.99));
    assert_ne!(uniform(10, 0), uniform(10, 1));
    assert_eq!(uniform(10, 0), uniform(10, 0));
}
