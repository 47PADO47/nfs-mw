//! Completed-tick observers must agree across input transitions.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::props::PropWorld;
use crate::settings::CarShading;
use crate::settings::xenon_effects::{XenonEffects, XenonEffectsLayer};
use blackbox_vehicle::VehicleSpec;
use nfsmw_data::car::physics::{CarBounds, SurfaceTable, WallSpec};
use nfsmw_data::car::{CarModel, TextureSwaps};
use nfsmw_data::vehicle_effects::{EmitterStyle, VisualEffectsData};

pub(super) fn fixture() -> (Drive, NullBackend) {
    let mut spec = VehicleSpec::example();
    spec.body.gravity = 0.0;
    spec.nos.nos_capacity = 120.0;
    spec.nos.torque_boost = 1.0;
    let physics = CarPhysics {
        effect_source: 0,
        bounds: CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO },
        spec,
        tire_integration: Default::default(),
        jump_config: Default::default(),
        walls: WallSpec::default(),
        geometry: None,
        events: None,
        reactions: None,
    };
    let model = CarModel {
        name: "TEST".into(),
        car_type: None,
        driving_lods: Default::default(),
        flare_profiles: Default::default(),
        solids: Default::default(),
        placements: Vec::new(),
        textures: Default::default(),
        swaps: TextureSwaps::default(),
        paint: None,
        window_tint_material: None,
        paintable: false,
        light_materials: Default::default(),
        floor_height: 0.0,
        corners: None,
        wheel_setup: None,
    };
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    let rig = CarRig::upload(&mut renderer, 123, model, CarShading::Original, Vec3::Z);
    let visuals = VisualEffectsData {
        trail: Some(EmitterStyle {
            color: [0.8, 1.0, 0.8, 0.15],
            // Previous particles expire before the next tick. Release must stop new births.
            life: STEP * 0.5,
            count: 15.0,
            velocity: [0.0, 0.0, -1.0],
            length: 200.0,
            height: 255.0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut drive = Drive::new(
        "TEST".into(),
        rig,
        physics.clone(),
        SpawnRequest { near: [0.0; 2], heading: None, exact: None, strict: false },
        None,
        visuals,
    );
    let mut sim = CarSim::new(physics, [0.2; 4]);
    sim.body_mut().place(Vec3::Y * 10.0, glam::Quat::IDENTITY);
    sim.body_mut().linear_velocity = Vec3::Z * 20.0;
    drive.current = sim.pose();
    drive.previous = drive.current;
    drive.telemetry = sim.telemetry();
    drive.gauges.snap(sim.gauge_sample());
    drive.sim = Some(sim);
    drive.request = None;
    drive.alpha = 1.0;
    drive.vehicle_effects.set_enabled(false, true);
    drive
        .vehicle_effects
        .set_tuning(XenonEffects::from_layer(XenonEffectsLayer { nos_contrails: Some(true), ..Default::default() }));
    (drive, renderer)
}

#[test]
fn nitrous_press_and_release_reach_effects_audio_hud_and_camera_in_the_completed_tick() {
    let (mut drive, mut renderer) = fixture();
    let collision = CollisionWorld::new(None);
    let mut props = PropWorld::default();
    let surfaces = SurfaceTable::default();
    for burning in [false, true, false, true, false] {
        drive.frame_input = DriveInput { throttle: 1.0, nos: burning, ..Default::default() };
        let previous = drive.current;
        let step = drive.step(&collision, &mut props, &surfaces).unwrap();
        let sim = drive.sim.as_ref().unwrap();
        let pose = sim.pose();
        let gauge = sim.gauge_sample();
        assert_eq!(sim.telemetry().nos_burning, burning, "fixture must cross the physical NOS state");
        drive.finish_step(&collision, &surfaces, step);

        let mut vertices = Vec::new();
        drive.vehicle_effects.geometry(Vec3::ZERO, Vec3::X, true, &mut vertices);
        assert_eq!(!vertices.is_empty(), burning, "current NOS controls new trail particles on press and release");
        assert_eq!(drive.tail_trails.samples().count() > 0, burning);
        assert_eq!(drive.nitrous_burning(), burning);
        assert_eq!(drive.original_body(false).unwrap().nos_engaged, burning);
        let hud = drive.hud_state().unwrap();
        assert_eq!(hud.telemetry.nos_burning, burning);
        assert_eq!((hud.rpm, hud.speed), (gauge.rpm, gauge.speed));
        let sound = drive.car_sound().unwrap();
        assert_eq!(sound.ticks.len(), 1, "each completed tick publishes exactly one audio tick");
        assert_eq!(sound.ticks[0].input.nos_active, burning);
        assert_eq!(drive.current, pose);
        drive.alpha = 0.0;
        assert_eq!(drive.pose(), previous, "render interpolation retains the preceding completed pose");
        drive.alpha = 1.0;
        assert_eq!(drive.pose(), pose);
    }
    drive.rig.release(&mut renderer);
}
