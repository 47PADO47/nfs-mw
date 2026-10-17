//! Installed marker facing and cached lamp mesh selection.

use super::rig::{CarPose, CarRig};
use crate::app::test_backend::NullBackend;
use crate::scenes::world::car_light_flares::FlareView;
use crate::scenes::world::car_lights::{CarLights, Kind, LampState};
use crate::settings::CarShading;
use blackbox_gfx::{Capabilities, GraphicsApi};
use glam::{Quat, Vec3};
use nfsmw_data::car::{LoadOptions, WheelPose};

#[test]
#[ignore = "requires an installed game"]
fn installed_elise_nitrous_trails_use_only_authored_brake_flares_and_current_brake_state() {
    use crate::scenes::world::tail_light_trails::TailLightTrails;
    let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let model =
        nfsmw_data::car::load(&dir, "ELISE", &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
    let lights = CarLights::for_model(&model);
    let tail_count = lights.lamps().iter().filter(|l| l.kind == Kind::Tail).count();
    let centre_count = lights.lamps().iter().filter(|l| l.kind == Kind::CentreBrake).count();
    assert!(tail_count >= 2);
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let flares = crate::scenes::world::car_light_flares::CarLightFlares::upload(&mut renderer, &model);
    let mut history = TailLightTrails::default();
    for i in 0..3 {
        history.step(glam::Mat4::from_translation(Vec3::X * i as f32 * 0.1), true, 20.0);
    }
    let view = FlareView { camera: -Vec3::X * 8.0 + Vec3::Z, forward: Vec3::X, fov_degrees: 70.0, height: 1080 };
    let mut coast = Vec::new();
    flares.draw_trails(&lights, &history, glam::Mat4::IDENTITY, LampState::default(), view, &mut coast);
    assert_eq!(coast.len(), model.flare_profiles.brake.len());
    assert_eq!(coast.iter().map(|b| b.vertices.len()).sum::<usize>(), 24 * tail_count * 6);
    let mut braking = Vec::new();
    flares.draw_trails(
        &lights,
        &history,
        glam::Mat4::IDENTITY,
        LampState { brake: 1.0, ..Default::default() },
        view,
        &mut braking,
    );
    assert_eq!(braking.iter().map(|b| b.vertices.len()).sum::<usize>(), 24 * (tail_count + centre_count) * 6);
    assert!(braking[0].vertices[0].color[3] >= coast[0].vertices[0].color[3]);
    let mut front = Vec::new();
    flares.draw_trails(
        &lights,
        &history,
        glam::Mat4::IDENTITY,
        LampState { headlights: true, ..Default::default() },
        FlareView { camera: Vec3::X * 8.0 + Vec3::Z, forward: -Vec3::X, ..view },
        &mut front,
    );
    assert!(front.is_empty(), "headlights and rear-facing brake markers cannot produce front trails");
    history.step(glam::Mat4::IDENTITY, false, 20.0);
    let mut released = Vec::new();
    flares.draw_trails(&lights, &history, glam::Mat4::IDENTITY, LampState::default(), view, &mut released);
    assert!(released.is_empty());
    flares.release(&mut renderer);
}

#[test]
#[ignore = "requires an installed game"]
fn installed_player_lamp_states_select_cached_meshes_and_respect_marker_facing() {
    let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    for car in ["BMWM3GTR", "CARRERAGT", "GALLARDO", "PUNTO"] {
        let model = nfsmw_data::car::load(&dir, car, &options).unwrap();
        assert_eq!(model.flare_profiles.head[0].color[3], 0.0, "installed inner head profile is invisible");
        assert!(model.flare_profiles.head[1].color[3] > 0.0, "installed outer head profile is visible");
        let visible_head_profiles = model.flare_profiles.head.iter().filter(|p| p.color[3] > 0.0).count();
        let lights = CarLights::for_model(&model);
        let heads: Vec<_> = lights.lamps().iter().filter(|l| l.kind == Kind::Head).collect();
        let tails: Vec<_> = lights.lamps().iter().filter(|l| l.kind == Kind::Tail).collect();
        assert!(heads.len() >= 2 && tails.len() >= 2, "{car}: assembled lamps");
        assert!(heads.iter().all(|l| l.direction.x > 0.0), "{car}: heads face forward");
        assert!(tails.iter().all(|l| l.direction.x < 0.0), "{car}: tails face rearward");
        let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan)).with_mesh_tracking();
        let rig = CarRig::upload(&mut renderer, 123, model, CarShading::default(), Vec3::Z);
        let meshes = renderer.mesh_labels.len();
        let pose = CarPose { position: Vec3::ZERO, rotation: Quat::IDENTITY, wheels: [WheelPose::default(); 4] };
        let mut states = Vec::new();
        for state in 0..4 {
            let mut instances = Vec::new();
            rig.instances_lit(123, &pose, None, &Default::default(), [state & 1 != 0, state & 2 != 0], &mut instances);
            states.push(instances);
        }
        assert_eq!(states[0].len(), states[3].len());
        // Stock PC cars can supply only HEADLIGHT_ON; the texture must stay available when toggled off.
        assert!(states[1].iter().zip(&states[3]).any(|(a, b)| a.mesh != b.mesh), "{car}: brake texture toggles");
        for fps in [30, 60, 144, 360] {
            for frame in 0..fps * 2 {
                let mut instances = Vec::new();
                rig.instances_lit(123, &pose, None, &Default::default(), [true, frame % 2 == 0], &mut instances);
                assert_eq!(instances.len(), states[1].len());
            }
        }
        assert_eq!(renderer.mesh_labels.len(), meshes, "drawing cannot upload new meshes");
        let state = LampState::default();
        let mut rear = Vec::new();
        rig.flares().draw(
            &lights,
            pose.transform(),
            state,
            FlareView { camera: -Vec3::X * 6.0, forward: Vec3::X, fov_degrees: 70.0, height: 1080 },
            &mut rear,
        );
        assert!(!rear.is_empty(), "{car}: rearward tail flares");
        let mut front = Vec::new();
        rig.flares().draw(
            &lights,
            pose.transform(),
            LampState { headlights: true, ..state },
            FlareView { camera: Vec3::X * 6.0, forward: -Vec3::X, fov_degrees: 70.0, height: 1080 },
            &mut front,
        );
        assert_eq!(front.len(), visible_head_profiles, "{car}: visible authored head flare batches");
        // The native road footprint belongs to an auxiliary reflection pass.
        // A normal-scene lamp quad must face the camera, never lie along the road.
        for batch in &front {
            for quad in batch.vertices.as_chunks::<6>().0 {
                let depth = quad[0].position[0];
                assert!(quad.iter().all(|v| (v.position[0] - depth).abs() < 0.0001), "{car}: scene has a road quad");
            }
        }
        let mut dark = Vec::new();
        rig.flares().draw(
            &lights,
            pose.transform(),
            state,
            FlareView { camera: Vec3::X * 6.0, forward: -Vec3::X, fov_degrees: 70.0, height: 1080 },
            &mut dark,
        );
        assert!(dark.is_empty(), "rear lamps must not glow through the front of the car");
        rig.release(&mut renderer);
        assert!(renderer.mesh_labels.is_empty());
    }
}
