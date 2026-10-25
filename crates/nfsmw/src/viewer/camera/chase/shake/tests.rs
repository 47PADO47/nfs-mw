use super::*;

#[test]
fn pulse_samples_match_the_original_pc_vectors() {
    let pulse = Pulse { start: 0.0, duration: 1.0, translation: Vec3::new(0.3, -0.2, 0.1) };
    // Synthetic inputs executed by the original PC sampler, no original asset bytes.
    for (phase, expected) in [
        (0.0, [0.3, -0.2, 0.1]),
        (0.03, [0.273, -0.182, 0.091]),
        (0.09, [0.0; 3]),
        (0.1, [0.0; 3]),
        (0.15, [0.0; 3]),
        (0.25, [0.165, -0.11, 0.055]),
        (0.3, [0.0; 3]),
        (0.4, [0.135, -0.09, 0.045]),
        (0.5, [0.0; 3]),
        (0.7, [0.072, -0.048, 0.024]),
        (0.9, [0.024, -0.016, 0.008]),
        (0.999, [0.0; 3]),
        (1.0, [0.0; 3]),
    ] {
        assert!((pulse.sample(phase) - Vec3::from(expected)).length() < 1e-7, "phase {phase}");
    }
}

#[test]
fn view_translation_and_rotations_match_the_original_matrix_application() {
    let view = Mat4::from_translation(Vec3::new(10.0, 20.0, 30.0));
    let expected = Mat4::from_cols_array(&[
        0.998_351_9,
        0.0,
        -0.05749256,
        0.0,
        0.0006614315,
        0.99994063,
        0.011485601,
        0.0,
        0.05748915,
        -0.011504644,
        0.9982855,
        0.0,
        12.026544,
        19.452536,
        29.683634,
        1.0,
    ]);
    let actual = apply(view, Vec3::new(0.3, -0.2, 0.1));
    assert!(actual.abs_diff_eq(expected, 0.0002), "{actual:?}");
}

#[test]
fn acceleration_bias_limits_and_retrigger_use_the_original_normalized_envelope() {
    let mut shake = Shake::default();
    shake.request(Vec3::X * 140.0); // Horizontal bias keeps this below the trigger.
    shake.advance(1.0 / 60.0);
    assert!(shake.current.is_none());
    shake.request(Vec3::Z * 100.0);
    shake.advance(1.0 / 60.0);
    let small = shake.current.unwrap();
    assert_eq!(small.translation, -Vec3::Z * 0.1);
    assert_eq!(small.duration, 0.3);
    shake.request(Vec3::Z * 1000.0);
    shake.advance(1.0 / 60.0);
    assert_eq!(shake.current.unwrap().start, small.start, "an early stronger hit cannot replace envelope 1");
    for _ in 0..12 {
        shake.advance(1.0 / 60.0);
    }
    shake.request(Vec3::Z * 1000.0);
    shake.advance(1.0 / 60.0);
    let large = shake.current.unwrap();
    assert_eq!(large.translation, -Vec3::Z * 0.4);
    assert!((large.duration - 0.7).abs() < 1e-6);
    assert!(large.start > small.start);
}

#[test]
fn presentation_never_starts_a_pulse_before_the_contact_tick() {
    let mut shake = Shake::default();
    shake.request(Vec3::Z * 600.0);
    shake.advance(1.0 / 60.0);
    assert_eq!(shake.sample(0.0), Vec3::ZERO);
    assert_eq!(shake.sample(0.99), Vec3::ZERO);
    assert_eq!(shake.sample(1.0), -Vec3::Z * 0.4);
    shake.advance(1.0 / 60.0);
    assert_eq!(shake.sample(0.0), -Vec3::Z * 0.4);
    assert!(shake.sample(1.0).length() < 0.4);
}

#[test]
fn the_intensity_scales_the_offset_and_zero_removes_it() {
    let hit = |intensity: f32| {
        let mut shake = Shake::default();
        shake.set_intensity(intensity);
        shake.request(Vec3::Z * 600.0);
        shake.advance(1.0 / 60.0);
        shake
    };
    assert_eq!(hit(1.0).sample(1.0), -Vec3::Z * 0.4, "1 keeps the original amplitude");
    assert!((hit(0.5).sample(1.0) - -Vec3::Z * 0.2).length() < 1e-6);
    assert_eq!(hit(0.0).sample(1.0), Vec3::ZERO, "0 is no shake");
    assert_eq!(hit(f32::NAN).sample(1.0), -Vec3::Z * 0.4, "a value that is not a number keeps the shake");
    assert_eq!(hit(3.0).sample(1.0), -Vec3::Z * 0.4, "clamped to 1");
}

#[test]
fn clearing_drops_the_pulses_and_keeps_the_intensity() {
    let mut shake = Shake::default();
    shake.set_intensity(0.25);
    shake.request(Vec3::Z * 600.0);
    shake.advance(1.0 / 60.0);
    shake.clear();
    assert_eq!(shake.sample(1.0), Vec3::ZERO);
    assert_eq!(shake.intensity, 0.25);
}
