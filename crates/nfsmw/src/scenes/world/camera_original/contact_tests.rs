//! Exercise the real player wall response before either camera consumes its inputs.

use blackbox_vehicle::{FlatGround, VehicleSpec};
use glam::Vec3;
use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};

use super::*;
use crate::scenes::world::drive::{CarSim, DriveInput};
use crate::scenes::world::{props::PropWorld, road::Spawn};
use crate::viewer::camera::{ChaseCamera, Followed};

fn car() -> CarSim {
    let spec = VehicleSpec::example();
    let bounds = CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO };
    CarSim::new(
        CarPhysics {
            effect_source: 0,
            spec,
            bounds,
            tire_integration: Default::default(),
            jump_config: Default::default(),
            walls: WallSpec::default(),
            geometry: None,
            events: None,
            reactions: None,
        },
        [0.2; 4],
    )
}

fn physical(car: &CarSim) -> PhysicsFrame {
    PhysicsFrame {
        body: car.original_camera_body(1.0, false),
        custom: None,
        avoidables: vec![],
        vehicles: vec![],
        noise: Default::default(),
        chopper_positions: vec![],
        explosions: vec![],
        impacts: vec![],
    }
}

fn view() -> nfsmw_data::camera::View {
    nfsmw_data::camera::View {
        fov: [90.0; 2],
        height: [1.7; 2],
        lag: [-5.44; 2],
        angle: [0.0; 2],
        stiffness: [0.5; 2],
        lat_offset: [0.0; 2],
        tilting: [true; 2],
        selectable: [true; 2],
    }
}

#[test]
fn player_wall_response_reaches_native_and_custom_contact_shakers() {
    for mode in [0, 1, 2, 3] {
        // The existing synthetic collision pack has a real wall at physics z=-3.
        let collision = super::tests::world(0);
        let ground = FlatGround::new(0.0);
        let mut props = PropWorld::default();
        let mut car = car();
        assert!(car.place_moving(&ground, Spawn { position: Vec3::new(-5.0, 0.0, 0.0), heading: 0.0 }, 30.0,));
        let mut native = original::Timeline::default();
        native.place(physical(&car));
        let mut custom = ChaseCamera::default();
        custom.update(Followed { position: car.pose().position, heading: 0.0, speed: 30.0 }, 1.0 / 60.0, |_, _| None);
        let impact = car.step(&DriveInput::default(), &ground, Some((&collision, &mut props)));
        assert!(impact.rigid > 0, "exercise a real world response, not injected camera acceleration");
        custom.impact(car.camera_acceleration());
        let followed =
            Followed { position: car.pose().position, heading: 0.0, speed: car.body().linear_velocity.length() };
        custom.update(followed, 1.0 / 60.0, |_, _| None);
        assert_ne!(custom.sample(1.0), custom.view(), "custom contact shake reaches presentation");
        native.physics_step(Some(physical(&car)));
        native.advance(
            Frame {
                real_dt: 2.0 / 60.0,
                sim_dt: 2.0 / 60.0,
                view: Some((mode, &view())),
                custom: false,
                aspect: 16.0 / 9.0,
                regions: None,
                speedbreaker: false,
                look_back: false,
            },
            |dt| dt,
            |_, _| None,
            |_, _, _| None,
        );
        assert!(native.acceleration().length() * 0.7 > 100.0, "the genuine wall slowdown exceeds native admission");
        assert!(native.has_shake(), "native view {mode} must receive the wall pulse");
    }
}

#[test]
fn camera_time_mapping_retains_the_physics_moment_scale_through_the_frame() {
    for active in [false, true] {
        for scale in [0.1, 0.25, 0.6, 1.0] {
            let ramp = SpeedbreakerClock::default();
            let real_dt = 0.2;
            let mut physical = ramp;
            let sim_dt = physical.advance(active, real_dt) * scale;
            for step in 0..=12 {
                let offset = real_dt * step as f32 / 12.0;
                let time = simulation_offset(ramp, active, offset, scale);
                assert!(time <= sim_dt + 1e-6, "camera must not consume contacts beyond this physics frame");
                let mut reference = ramp;
                let expected = reference.advance(active, offset) * scale;
                assert!((time - expected).abs() < 1e-6, "retain the nonlinear Speedbreaker ramp");
            }
            assert_eq!(simulation_offset(ramp, active, real_dt, scale), sim_dt);
        }
    }
}
