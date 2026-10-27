use super::*;
use blackbox_gfx::Projection;
use glam::Mat4;

fn frame() -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: -Vec3::new(0.7, 0.0, 0.7).normalize(),
        clear_color: [0.55, 0.65, 0.75],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}

fn evaluate(coefficients: [[f32; 4]; 9], n: Vec3) -> Vec3 {
    coefficients.into_iter().zip(basis(n)).map(|(c, b)| Vec3::from_array(c[..3].try_into().unwrap()) * b).sum()
}

#[test]
fn irradiance_has_direction_without_camera_or_wind_dependence() {
    let mut frame = frame();
    let mut sky = SkyState { enhanced: true, ..Default::default() };
    let original = coefficients(&frame, Some(sky));
    let up = evaluate(original, Vec3::Z);
    let down = evaluate(original, Vec3::NEG_Z);
    assert!(up.element_sum() > down.element_sum() * 1.1, "sky {up:?}, ground {down:?}");
    assert!(up.z / up.x > down.z / down.x, "sky should be cooler than ground");
    frame.camera_position = Vec3::splat(1234.0);
    frame.view = Mat4::from_rotation_z(1.0);
    sky.scroll = 5000.0;
    assert_eq!(coefficients(&frame, Some(sky)), original);
}

#[test]
fn weather_and_sun_change_the_field_continuously() {
    let mut frame = frame();
    let mut sky = SkyState { enhanced: true, ..Default::default() };
    let clear = coefficients(&frame, Some(sky));
    sky.cloud_coverage = 1.0;
    assert_ne!(clear, coefficients(&frame, Some(sky)));
    sky.cloud_coverage = 0.0;
    frame.light_dir = -Vec3::new(1.0, 0.0, 0.15).normalize();
    let dusk = coefficients(&frame, Some(sky));
    assert_ne!(clear, dusk);
    frame.light_dir.x += 0.0001;
    let next = coefficients(&frame, Some(sky));
    assert!(dusk.iter().flatten().zip(next.iter().flatten()).all(|(a, b)| (a - b).abs() < 0.002));
    for coverage in [0.0, 0.5, 1.0, f32::NAN] {
        sky.cloud_coverage = coverage;
        assert!(coefficients(&frame, Some(sky)).iter().flatten().all(|v| v.is_finite()));
    }
}

#[test]
fn directional_field_preserves_mean_authored_energy() {
    let c = coefficients(&frame(), Some(SkyState { enhanced: true, ..Default::default() }));
    let mean = (0..SAMPLES).map(|i| evaluate(c, direction(i, SAMPLES))).sum::<Vec3>() / SAMPLES as f32;
    assert!((mean - Vec3::ONE).abs().max_element() < 0.01, "preserve every authored ambient channel: {mean:?}");
}
