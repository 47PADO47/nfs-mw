//! Installed AI cars and trailers reach authored flare output through their actor adapters.

use super::*;
use crate::scenes::world::ai::car::{AiCar, Role, Start};
use crate::scenes::world::ai::traffic::Cruise;
use crate::scenes::world::car_light_flares::FlareView;
use crate::scenes::world::car_lights::{Kind, LampState};
use blackbox_roads::RoadNav;

fn view(pose: CarPose, front: bool) -> FlareView {
    let along = pose.rotation * Vec3::X;
    let side = match front {
        true => 1.0,
        false => -1.0,
    };
    FlareView {
        camera: pose.position + along * (20.0 * side) + Vec3::Z,
        forward: -along * side,
        fov_degrees: 70.0,
        height: 1080,
    }
}

#[test]
#[ignore = "needs installed traffic/police models, light profiles and highway ground"]
fn installed_ai_lamps_use_authored_profiles_actual_brakes_and_directional_markers() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let (mut spawn, _, _) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan)).with_mesh_tracking();
    let mut mesh_witnesses = 0;
    for name in ["traf4dseda", "copmidsize"] {
        let model = model(&dir, &data, name, &mut renderer);
        let mut car = AiCar::place(
            &model,
            Role::Traffic,
            RoadNav::traffic(),
            spawn,
            Start { speed: 0.0, stagger: 0, cruise: Cruise::default(), patrol: false },
            &ground,
        )
        .unwrap();
        let mut coasting = Vec::new();
        car.instances_with_detail(1.0, None, &mut coasting);
        let mut props = PropWorld::default();
        for tick in 0..32 {
            car.sims_mut().0.step(
                &DriveInput { brake: 1.0, ..Default::default() },
                &ground,
                Some((&collision, &mut props)),
            );
            if tick == 0 {
                assert_eq!(car.sims_mut().0.lamp_brake(), 0.0, "NPC lamps do not flash for one-tick corrections");
            }
        }
        assert_eq!(car.sims_mut().0.lamp_brake(), 1.0, "sustained NPC braking admits lamps after the native delay");
        car.sync_contacts();
        let pose = car.pose(1.0);
        let rear = view(pose, false);
        let mut actual = Vec::new();
        car.flares(1.0, rear, &mut actual);
        let mut expected = Vec::new();
        model.rig.flares().draw(
            model.rig.lights(),
            pose.transform(),
            LampState {
                headlights: true,
                brake: 1.0,
                time: car.lamp_time(1.0),
                emergency: match car.emergency_lights() {
                    true => 0x7000,
                    false => 0,
                },
                ..Default::default()
            },
            rear,
            &mut expected,
        );
        assert!(!actual.is_empty(), "{name}: the actor must submit installed tail flares");
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(&expected) {
            assert_eq!(actual.texture, expected.texture);
            assert_eq!(actual.blend, expected.blend);
            assert_eq!(
                actual.vertices, expected.vertices,
                "{name}: current physical brake input reaches authored output"
            );
        }
        let mut braked = Vec::new();
        car.instances_with_detail(1.0, None, &mut braked);
        mesh_witnesses += usize::from(coasting.iter().zip(&braked).any(|(a, b)| a.mesh != b.mesh));
        let uploaded = renderer.mesh_labels.len();
        for _ in 0..100 {
            let mut instances = Vec::new();
            car.instances_with_detail(1.0, None, &mut instances);
        }
        assert_eq!(renderer.mesh_labels.len(), uploaded, "lamp changes select cached meshes");
        let mut front = Vec::new();
        car.flares(1.0, view(pose, true), &mut front);
        assert!(!front.is_empty(), "{name}: forward-facing authored headlights");
        let mut far = Vec::new();
        car.flares(1.0, FlareView { camera: rear.camera - rear.forward * 250.0, ..rear }, &mut far);
        assert!(far.is_empty(), "{name}: native range gate applies to traffic too");
    }
    assert!(mesh_witnesses > 0, "at least one installed model must expose distinct brake surface variants");
}

#[test]
#[ignore = "needs installed trailer geometry/profiles and highway ground; adds an explicit synthetic lamp probe"]
fn installed_trailer_lamps_follow_their_own_body_and_brakes_after_release() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let (mut spawn, _, _) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    let tractor_model = model(&dir, &data, "semib", &mut renderer);
    let trailer_model = model(&dir, &data, &data.trailer_of("semib").unwrap(), &mut renderer);
    let mut tractor = CarSim::for_rig(tractor_model.physics.clone(), &tractor_model.rig);
    tractor.configure_traffic();
    assert!(tractor.place_traffic_moving(&ground, spawn, 0.0));
    let mut trailer = TrailerCar::place(&trailer_model, &tractor, spawn, 0.0, &ground).unwrap();
    assert!(
        !trailer_model.rig.lights().lamps().iter().any(|lamp| lamp.kind == Kind::Tail),
        "stock TRAILERB has no authored tail flare markers; do not invent them"
    );
    let mut props = PropWorld::default();
    for _ in 0..32 {
        trailer.step(&mut tractor, Brakes { brake: 1.0, handbrake: false }, &collision, &mut props, &data.surfaces);
    }
    let rear = view(trailer.pose(1.0), false);
    let mut authored = Vec::new();
    trailer.flares(1.0, rear, &mut authored);
    assert!(authored.is_empty(), "a stock trailer without markers must not gain inferred flare positions");
    assert_eq!(trailer.sim.lamp_brake(), 1.0);
    // The installed geometry cannot witness marker-driven trailer output because it has no
    // lamp markers. Add one explicitly synthetic marker to exercise the actor adapter while
    // retaining the real trailer controls, pose, installed profiles and textures.
    let mut probe =
        nfsmw_data::car::load(&dir, "TRAILERB", &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
    let placement = probe.placements.iter().find(|p| p.corner.is_none()).unwrap();
    let lamp = glam::Mat4::from_cols(
        Vec3::Y.extend(0.0),
        Vec3::NEG_Z.extend(0.0),
        Vec3::NEG_X.extend(0.0),
        Vec3::new(-4.0, 0.0, 1.0).extend(1.0),
    );
    probe.solids.get_mut(&placement.solid).unwrap().markers.push(blackbox_solid::PositionMarker {
        name_hash: blackbox_hash::bstring_hash("LEFT_BRAKELIGHT"),
        int_param: 0,
        float_params: [0.0; 2],
        matrix: (placement.transform.inverse() * lamp).to_cols_array(),
    });
    trailer.rig = Rc::new(CarRig::upload(&mut renderer, 999, probe, CarShading::default(), sun::to_sun()));
    assert_eq!(trailer.rig.lights().lamps().iter().filter(|lamp| lamp.kind == Kind::Tail).count(), 1);
    let mut braking = Vec::new();
    trailer.flares(1.0, rear, &mut braking);
    assert!(
        !braking.is_empty(),
        "the synthetic marker reaches installed-profile flare output through the real trailer adapter"
    );
    let supported = Posture { up: Vec3::Y, wheels_on_ground: 4 };
    trailer.coupling.hitch.update(STEP, supported, Posture { up: Vec3::X, ..supported });
    assert!(!trailer.is_hitched());
    trailer.step(&mut tractor, Brakes { brake: 1.0, handbrake: true }, &collision, &mut props, &data.surfaces);
    assert_eq!(trailer.sim.lamp_brake(), 0.0, "former tractor braking cannot keep the trailer lamps bright");
    let mut released = Vec::new();
    trailer.flares(1.0, view(trailer.pose(1.0), false), &mut released);
    assert!(!released.is_empty(), "the released trailer keeps the synthetic probe at its own pose");
    assert!(released[0].vertices[0].color[3] < braking[0].vertices[0].color[3]);
}
