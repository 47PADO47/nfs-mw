//! Exercise the scene's real startup wiring, rather than constructing configured catalogs in fixtures.

use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

use super::super::{Options, View, WorldScene};

mod lamps;
mod materials;
mod reflections;
mod shadow_categories;
mod traffic;

fn open() -> (GameDir, WorldScene) {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let scene = WorldScene::open(
        &dir,
        Options {
            at: None,
            height: 40.0,
            heading: 0.0,
            pitch: -15.0,
            fog_distance: 1500.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    (dir, scene)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_scene_startup_connects_vehicle_damage_sequences() {
    let (_, scene) = open();
    for name in ["trailerlog", "trailerb"] {
        let car = scene.physics.car(name).unwrap();
        assert!(
            car.events.as_ref().is_some_and(|events| events.sequence.is_some()),
            "scene startup must connect {name} to its installed cargo sequencer"
        );
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_scene_startup_connects_scenery_fragment_hierarchies() {
    let (dir, scene) = open();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert!(!index.model_hierarchies.is_empty(), "fixture must have installed model hierarchies");
    for hierarchy in index.model_hierarchies {
        assert!(
            scene.residency.catalog.hierarchy(hierarchy.name_hash).is_some(),
            "scene startup must retain scenery hierarchy {:08X}",
            hierarchy.name_hash
        );
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_cargo_damage_uses_the_active_camera_after_leaving_the_free_camera() {
    use crate::app::test_backend::NullBackend;
    use crate::scenes::world::events::{DamageHit, OtherBody, VehicleEvents};
    use crate::viewer::camera::Camera;
    use blackbox_collision::CollisionWorld;
    use blackbox_gfx::{Capabilities, GraphicsApi};
    use glam::Vec3;
    use nfsmw_data::world::sequences::WorldEvent;

    let (dir, mut scene) = open();
    let trailer = scene.physics.car("trailerlog").unwrap();
    let mut spec = trailer.events.unwrap();
    eprintln!("TRAILERLOG authored suppress distance: {} m", spec.suppress_distance);
    // Exercise the generic distance gate without changing the installed trailer's tuning.
    std::sync::Arc::make_mut(&mut spec).suppress_distance = 50.0;
    scene.camera.position = Vec3::X * (spec.suppress_distance + 1000.0);
    let (_, model) = super::super::load_car(&dir, "BMWM3GTR").unwrap();
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    scene.start_driving(&mut renderer, "BMWM3GTR".into(), model, None).unwrap();
    scene.players.lead_mut().unwrap().follow(&CollisionWorld::new(None), (0.0, 0.0), 0.0);
    let chase = scene.players.lead().unwrap().physics_camera();
    let fragments = |distance: f32| {
        let mut events = VehicleEvents::new(spec.clone()).unwrap();
        events.hit(DamageHit {
            arm: Vec3::NEG_Z * trailer.bounds.half_dimensions.z,
            normal: Vec3::Z,
            half: trailer.bounds.half_dimensions,
            closing_speed: 50.0,
            force: spec.strength * 10.0,
            shock_force: 0.0,
            before_velocity: Vec3::Z * 50.0,
            other: OtherBody { vehicle: true, player: true, ..Default::default() },
        });
        (0..240)
            .flat_map(|_| events.tick(distance))
            .filter(|effect| matches!(effect.event, WorldEvent::Fragment(_)))
            .count()
    };
    let active = scene.simulation_lens(scene.physics_aspect);
    assert_eq!(active.position, chase.position, "driving must not retain the parked free camera");
    assert_eq!(active.view_proj(), chase.lens(scene.physics_aspect).view_proj());
    assert!(active.position.length() < spec.suppress_distance);
    assert_eq!(fragments(active.position.length()), 9, "nearby damage must emit the installed fragment events");
    scene.view = View::Fly;
    let active = scene.simulation_lens(scene.physics_aspect);
    assert_eq!(active.position, scene.camera.position);
    assert_eq!(fragments(active.position.length()), 0, "a distant free camera retains suppression");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_simulation_camera_honours_every_selected_view_without_render_interpolation() {
    use crate::app::test_backend::NullBackend;
    use crate::scenes::world::{ai::PlayerView, props::PropWorld, road::Spawn};
    use crate::settings::CameraMode;
    use crate::viewer::camera::{Camera, DynamicFov, original};
    use blackbox_gfx::{Capabilities, GraphicsApi};
    use glam::Vec3;

    let (dir, mut scene) = open();
    let (_, model) = super::super::load_car(&dir, "BMWM3GTR").unwrap();
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    scene.start_driving(&mut renderer, "BMWM3GTR".into(), model, None).unwrap();
    assert!(scene.camera_environment.views.is_some(), "fixture must have all installed camera views");
    let collision = flat_road();
    let drive = scene.players.lead_mut().unwrap();
    assert!(drive.spawn(Spawn { position: Vec3::ZERO, heading: 0.0 }, &collision, &scene.physics.surfaces));
    drive.follow(&collision, (0.0, 0.0), 0.0);
    let sim = drive.sim_mut().unwrap();
    sim.body_mut().position += Vec3::Z * 10.0;
    sim.body_mut().linear_velocity = Vec3::Z * 25.0;
    let step = drive.step(&collision, &mut PropWorld::default(), &scene.physics.surfaces).unwrap();
    drive.finish_step(&collision, &scene.physics.surfaces, step);
    drive.follow(&collision, (0.0, 0.0), super::super::drive::STEP);
    // Contacts can move the body before its next render-pose publication.
    drive.sim_mut().unwrap().body_mut().position += Vec3::Z * 2.0;
    let physical = drive.physics_pose();
    drive.set_alpha(1.0);
    assert!(physical.position.distance(drive.pose().position) > 1.0, "test must exercise the live body pose");
    let player = PlayerView {
        nos_input: false,
        nitrous_boost: 0.0,
        velocity: (physical.rotation * Vec3::X) * (60.0),
        position: physical.position,
        forward: physical.rotation * Vec3::X,
        speed: 60.0,
        nitrous: 1.0,
        throttle: 1.0,
    };
    scene.traffic.as_mut().unwrap().update_police(1.0, &player, &|_, _| true);
    let kick = scene.traffic.as_ref().unwrap().fov_kick();
    assert!(kick > 0.0, "the real high-speed effect path must supply a nitrous FOV kick");
    let aspect = scene.physics_aspect;
    for mode in CameraMode::ALL {
        scene.camera_mode = mode;
        scene.original_player_step = None;
        scene.seed_original_camera();
        scene.original_camera.reset();
        scene.update_original_camera(super::super::drive::STEP, 0.0, Default::default(), false, 1.0);
        let drive = scene.players.lead().unwrap();
        let chase = drive.physics_camera();
        let expected = match original::index_of(mode) {
            Some(_) => scene.original_camera.lens(drive.original_body(false).unwrap(), aspect, false).unwrap(),
            None if mode == CameraMode::DynamicFov => {
                DynamicFov { chase: &chase, speed: drive.speed().abs(), kick, intensity: 1.0 }.lens(aspect)
            }
            None => chase.lens_with_fov(aspect, chase.fov_degrees() + kick),
        };
        for alpha in [0.0, 0.25, 0.5, 1.0] {
            scene.players.lead_mut().unwrap().set_alpha(alpha);
            let actual = scene.simulation_lens(aspect);
            assert!((actual.position - expected.position).length() < 1e-4, "{mode:?} at render alpha {alpha}");
            assert!(actual.view.abs_diff_eq(expected.view, 1e-4), "{mode:?} view at render alpha {alpha}");
            assert!(actual.projection.matrix().abs_diff_eq(expected.projection.matrix(), 1e-4), "{mode:?} projection");
        }
        scene.players.lead_mut().unwrap().set_alpha(0.0);
        let before = scene.gameplay_lens(aspect);
        scene.players.lead_mut().unwrap().set_alpha(1.0);
        assert_ne!(scene.gameplay_lens(aspect).position, before.position, "{mode:?} must still interpolate");
    }
    scene.view = View::Fly;
    let expected = scene.camera.lens(aspect);
    for mode in CameraMode::ALL {
        scene.camera_mode = mode;
        assert_eq!(scene.simulation_lens(aspect), expected, "{mode:?} must preserve the free camera");
        assert_eq!(scene.gameplay_lens(aspect), expected);
    }
}

/// A synthetic road keeps the camera test independent of streaming and the installed road layout.
fn flat_road() -> blackbox_collision::CollisionWorld {
    use blackbox_collision::{Article, CollisionPack, CollisionWorld, Instance, PackedVert, STRIP_UP_FACING, Strip};

    let verts = [(-8192, -8192), (-8192, 8192), (8192, -8192), (8192, 8192)].map(|(x, z)| PackedVert {
        x,
        y: 0,
        z,
        surface: 0,
        flags: 0,
    });
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack {
        section: 0,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 64.0,
            half_length: 64.0,
            half_height: 1.0,
            radius: 91.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![Strip { center: [0.0; 3], radius: 91.0, flags: STRIP_UP_FACING, verts: verts.to_vec() }],
            barriers: Vec::new(),
            surfaces: vec![blackbox_attrib::vlt_hash("asphalt")],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: Vec::new(),
    });
    world
}
