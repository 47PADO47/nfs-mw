use super::*;

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
