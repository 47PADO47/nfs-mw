use super::*;
const TREADS: [Vec3; 4] = [
    Vec3::new(1.4, 0.8, -0.37),
    Vec3::new(1.4, -0.8, -0.37),
    Vec3::new(-1.3, -0.75, -0.34),
    Vec3::new(-1.3, 0.75, -0.34),
];
#[test]
fn four_treads_follow_grade_and_bank_at_any_heading() {
    let spot = CarSpot {
        position: Vec3::new(12.0, 20.0, 3.0),
        rotation: Quat::from_euler(glam::EulerRot::ZYX, 1.6, -0.5, 0.2),
    };
    let height = |p: Vec3| (p.x - 12.0) * 0.13 - (p.y - 20.0) * 0.2 + 3.0;
    let (placed, travel) = place(spot, TREADS, 1.0, 3.0, |p, _| Some(height(p))).unwrap();
    assert!((heading_of(placed.rotation) - heading_of(spot.rotation)).abs() < 0.0001);
    assert_eq!(spot.position.truncate(), placed.position.truncate());
    for (p, t) in TREADS.into_iter().zip(travel) {
        let at = placed.position + placed.rotation * (p + Vec3::Z * t);
        assert!((at.z - height(at)).abs() < 0.002);
    }
}
#[test]
fn three_contacts_support_a_car_at_a_bridge_edge() {
    let spot = CarSpot { position: Vec3::Z * 10.0, rotation: Quat::IDENTITY };
    let (placed, travel) = place(spot, TREADS, 1.0, 10.0, |p, reach| {
        let road = if p.x > 0.0 && p.y > 0.0 { 3.0_f32 } else { 10.0 };
        ((road - 10.0).abs() <= reach).then_some(road)
    })
    .unwrap();
    assert!((placed.position.z - 10.35).abs() < 0.03);
    assert_eq!(travel[0], 0.0);
    assert!(place(spot, TREADS, 1.0, 10.0, |_, _| None).is_none());
}
#[test]
#[ignore = "requires installed car models and physics"]
fn installed_scene_treads_across_the_car_catalog() {
    use crate::app::test_backend::NullBackend;
    use crate::scenes::world::drive::{CarRig, CarSim};
    use crate::settings::CarShading;
    use clap::Parser;
    use nfsmw_data::car::{self, LoadOptions, physics::PhysicsData};
    let path = std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap());
    let (dir, _mods) = match std::env::var_os("NFSMW_TEST_MODS_DIR") {
        Some(root) => {
            let cli = crate::cli::Cli::parse_from([
                std::ffi::OsString::from("nfsmw"),
                "view-world".into(),
                "--mods-dir".into(),
                root,
            ]);
            let Some(crate::cli::Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
            let (dir, mods) = crate::modding::open_install(Some(&path), &view).unwrap();
            (dir, Some(mods))
        }
        None => (game_install::GameDir::open(path).unwrap(), None),
    };
    let tables = car::CarTables::load(&dir).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let mut checked = 0;
    for name in car::list(&dir) {
        let kind = tables.car_type_for_folder(&name);
        if kind.is_some_and(|c| c.usage_type > 2) {
            continue;
        }
        let physics = data.car(&name).or_else(|error| {
            let kind = kind.ok_or(error)?;
            data.car(&kind.type_name).or_else(|_| data.car(&kind.base_model_name))
        });
        let Ok(physics) = physics else {
            eprintln!("no vehicle physics: {name}");
            continue;
        };
        let model = car::load_driving(&dir, &name, &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        let rig = CarRig::upload(&mut renderer, 1, model, CarShading::Original, Vec3::Z);
        let sim = CarSim::for_rig(physics, &rig);
        let (points, scale, visual) = rig.scene_treads(&sim, sim.pose());
        let spot = CarSpot { position: Vec3::ZERO, rotation: Quat::from_rotation_y(-0.45) };
        let height = |p: Vec3| 0.12 * p.x - 0.08 * p.y;
        let (placed, travel) = place(spot, points, scale, 0.0, |p, _| Some(height(p))).unwrap();
        let gap = points
            .into_iter()
            .zip(travel)
            .map(|(p, t)| {
                let at = placed.position + placed.rotation * (p + Vec3::Z * t * scale);
                (at.z - height(at)).abs()
            })
            .fold(0.0_f32, f32::max);
        assert!(gap < 0.01, "{name}: gap {gap}");
        eprintln!("catalog tread check {name} mesh={visual} max_gap_m={gap:.5}");
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn released_car_keeps_lift_and_roll_and_clears_a_buried_tread() {
    let mut spot = CarSpot { position: Vec3::new(3.0, 4.0, 0.15), rotation: Quat::from_rotation_x(0.04) };
    let mut travel = [-0.25; 4];
    let rotation = spot.rotation;
    released(&mut spot, &mut travel, TREADS, 1.0, |_| Some(0.0));
    assert_eq!(spot.rotation, rotation);
    for (p, t) in TREADS.into_iter().zip(travel) {
        let at = spot.position + rotation * (p + Vec3::Z * t);
        assert!(at.z.abs() < 1e-5);
    }
    spot.position.z += 1.8;
    let before = spot.position;
    let old = travel;
    released(&mut spot, &mut travel, TREADS, 1.0, |_| Some(0.0));
    assert_eq!(spot.position, before);
    assert_eq!(travel, old);
}

#[test]
fn buried_channel_uses_a_nearby_confirmed_street_deck() {
    let at = Vec3::new(1729.0, -174.0, 91.2);
    let street = at + Vec3::new(0.0, 2.3, 4.6);
    let probe = |top: f32, bottom: f32| (bottom <= 95.9 && top >= 95.9).then_some(95.9);
    assert_eq!(mapped_ground(at, street, probe), Some(95.9));
    assert_eq!(mapped_ground(at, street + Vec3::X * 50.0, probe), None);
    assert_eq!(mapped_ground(at, street + Vec3::Z * 10.0, probe), None);
    assert_eq!(mapped_ground(at, street, |_, _| None), None);
}
