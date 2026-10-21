//! Actual installed lamp ownership and explicit destruction, without guessed damage-zone thresholds.

use super::*;
use crate::scenes::world::ai::car::{AiCar, Role, Start};
use crate::scenes::world::ai::traffic::Cruise;
use crate::scenes::world::car_light_flares::FlareView;
use crate::scenes::world::car_lights::LampState;
use blackbox_roads::RoadNav;

fn front(pose: CarPose) -> FlareView {
    let forward = pose.rotation * Vec3::X;
    FlareView { camera: pose.position + forward * 20.0 + Vec3::Z, forward: -forward, fov_degrees: 70.0, height: 1080 }
}

#[test]
#[ignore = "needs installed police models, lamp profiles and highway ground"]
fn installed_destroyed_police_lamps_are_dark_and_not_reclassified_from_panel_damage() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let (mut spawn, _, _) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let model = model(&dir, &data, "copmidsize", &mut renderer);
    let mut car = AiCar::place(
        &model,
        Role::Traffic,
        RoadNav::traffic(),
        spawn,
        Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: true },
        &ground,
    )
    .unwrap();
    let view = front(car.pose(1.0));
    let mut before = Vec::new();
    car.flares(1.0, view, &mut before);
    assert!(!before.is_empty(), "real headlights must be visible before destruction");
    let sim = car.sims_mut().0;
    sim.events.as_mut().unwrap().damage.impact(0, 6, true);
    assert_eq!(sim.lamp_damage(), 0, "a cosmetic panel/glass level cannot invent a broken-light mask");
    assert!(car.destroy_by_breaker(), "use the actual police destruction path");
    assert_eq!(car.sims_mut().0.lamp_damage(), u32::MAX);
    let mut after = Vec::new();
    car.flares(1.0, view, &mut after);
    assert!(after.is_empty(), "the actor's destroyed state must reach authored lamp output");
}

#[test]
#[ignore = "needs installed vehicle markers, profiles and textures"]
fn installed_broken_lamp_bits_preserve_the_opposite_side_and_suppress_trail_copies() {
    use crate::scenes::world::tail_light_trails::TailLightTrails;
    use glam::{Mat4, Quat};
    use nfsmw_data::car::WheelPose;

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model =
        nfsmw_data::car::load(&dir, "BMWM3GTR", &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let rig = CarRig::upload(&mut renderer, 1234, model, CarShading::default(), sun::to_sun());
    let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [WheelPose::default(); 4] };
    let draw = |broken| {
        let mut out = Vec::new();
        rig.flares().draw(
            rig.lights(),
            pose.transform(),
            LampState { headlights: true, broken, ..Default::default() },
            front(pose),
            &mut out,
        );
        out.iter().map(|batch| batch.vertices.len()).sum::<usize>()
    };
    let normal = draw(0);
    assert!(normal >= 12);
    assert_eq!(draw(1) * 2, normal, "left head flag must leave the right head lit");
    assert_eq!(draw(2) * 2, normal, "right head flag must leave the left head lit");
    assert_eq!(draw(3), 0);
    let mut history = TailLightTrails::default();
    for step in 0..3 {
        history.step(Mat4::from_translation(Vec3::X * step as f32), true, 20.0);
    }
    let view = FlareView { camera: -Vec3::X * 20.0 + Vec3::Z, forward: Vec3::X, ..front(pose) };
    let mut normal_trails = Vec::new();
    rig.flares().draw_trails(rig.lights(), &history, pose.transform(), LampState::default(), view, &mut normal_trails);
    assert!(!normal_trails.is_empty(), "the fixture must expose ordinary native trail copies");
    let mut broken_trails = Vec::new();
    rig.flares().draw_trails(
        rig.lights(),
        &history,
        pose.transform(),
        LampState { broken: u32::MAX, ..Default::default() },
        view,
        &mut broken_trails,
    );
    assert!(broken_trails.is_empty());
    rig.release(&mut renderer);
}
