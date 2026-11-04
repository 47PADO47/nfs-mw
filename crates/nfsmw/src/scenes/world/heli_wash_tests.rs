use blackbox_particles::EmitterSpec;

use super::*;

#[test]
fn the_wash_frame_stands_on_the_ground_with_z_along_the_upward_normal() {
    let flat = wash_frame(Vec3::new(1.0, 2.0, 3.0), Vec3::NEG_Y);
    assert!(flat.z_axis.truncate().abs_diff_eq(Vec3::Z, 1e-6), "a downward normal is turned up");
    assert!(flat.x_axis.truncate().abs_diff_eq(Vec3::X, 1e-6), "flat ground leaves the frame unturned");
    assert!(flat.w_axis.truncate().abs_diff_eq(space::to_render([1.0, 2.0, 3.0]), 1e-6));
    let slope = Vec3::new(0.3, 1.0, 0.0).normalize();
    let sloped = wash_frame(Vec3::ZERO, slope);
    assert!(sloped.z_axis.truncate().abs_diff_eq(space::to_render(slope.to_array()), 1e-6));
}

#[test]
fn the_dust_blows_out_along_flat_ground_not_up() {
    // The install's dust emitter: 50 m/s, a 10 degree disc, quadratic drag 0.2.
    let dust = EmitterSpec {
        rate: 600.0,
        life: 2.0,
        speed: 50.0,
        spread: 10.0,
        spread_as_disc: true,
        drag: 0.2,
        ..Default::default()
    };
    let mut emitter = Emitter::new(dust, 1);
    let frame = Frame { to_world: wash_frame(Vec3::ZERO, Vec3::Y), inherit_velocity: Vec3::ZERO, intensity: 1.0 };
    for _ in 0..60 {
        emitter.step(1.0 / 60.0, &frame);
    }
    let out = emitter.sprites().map(|s| s.position.truncate().length()).fold(0.0, f32::max);
    let up = emitter.sprites().map(|s| s.position.z.abs()).fold(0.0, f32::max);
    assert!(out > 8.0, "spreads {out} m across the ground");
    assert!(up < 0.1 * out, "stays low: {up} m against {out} m out");
}

#[test]
fn the_wash_is_full_to_five_metres_fades_to_thirty_and_is_paused_above() {
    assert_eq!(intensity(0.0), Some(1.0));
    assert_eq!(intensity(5.0), Some(1.0));
    assert!((intensity(17.5).unwrap() - 0.5).abs() < 1e-6);
    assert_eq!(intensity(30.0), Some(0.0));
    assert_eq!(intensity(30.1), None);
    assert_eq!(intensity(f32::NAN), None);
}

#[test]
fn a_wash_without_art_does_nothing() {
    let mut wash = HeliWash::default();
    wash.step(1.0 / 60.0, Some(Vec3::new(0.0, 10.0, 0.0)), &CollisionWorld::new(None));
    assert_eq!(wash.current_intensity(), 0.0);
    let mut out = Vec::new();
    wash.textured(Vec3::ZERO, Vec3::X, &mut out);
    assert!(out.is_empty());
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_wash_loads_its_three_emitters_and_textures() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let data = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    let mut wash = HeliWash::default();
    wash.load(&mut renderer, &dir, data.database());
    assert_eq!(wash.emitters.len(), 3, "the install's heliwash group");
}
