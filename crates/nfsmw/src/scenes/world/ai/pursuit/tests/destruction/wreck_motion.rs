//! Actual installed cops retain native coasting controls, then enter supported wreck sleep.

use super::*;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shapes};
use blackbox_vehicle::rigid_body::BodyState;

fn approaching_striker(striker: &mut CarSim, cop: &CarSim) {
    let forward = cop.body().rotation().z_axis;
    let radius = cop.collision_radius() + striker.collision_radius();
    let samples = (radius / 0.025).ceil() as usize;
    for sample in 0..=samples {
        // Search the actual installed padded primitives, rather than assuming body half-length
        // locates a rear bumper. Only the striker moves; the sleeping cop is never repositioned.
        let distance = radius * (1.0 - sample as f32 / samples as f32);
        striker.body_mut().place(cop.body().position - forward * distance, cop.body().orientation);
        striker.body_mut().linear_velocity = forward * 3.0;
        let previous = (striker.body().position, striker.body().orientation);
        // This production step refreshes the sweep origin after direct test placement.
        striker.step(&DriveInput::default(), &NoGround, None);
        let first = object_shapes(
            striker.body(),
            striker.geometry.as_ref().map(|g| BodyGeometryFrame { geometry: &g.body, previous }),
        );
        let second = object_shapes(
            cop.body(),
            cop.geometry.as_ref().map(|g| BodyGeometryFrame {
                geometry: &g.body,
                previous: (cop.body().position, cop.body().orientation),
            }),
        );
        let contact = first
            .iter()
            .flat_map(|a| second.iter().filter_map(move |b| a.contact(b)))
            .max_by(|a, b| a.overlap.total_cmp(&b.overlap));
        if let Some(hit) = contact {
            let relative = striker.body().point_velocity(hit.point) - cop.body().point_velocity(hit.point);
            if hit.overlap > 0.0 && relative.dot(hit.normal) < -0.1 {
                eprintln!(
                    "wake witness: sample {sample}/{samples}, distance {distance:.4}, shapes {}/{}, positions {:?}/{:?}, half dimensions {:?}/{:?}, overlap {:.5}, normal {:?}, closing {:.5}",
                    first.len(),
                    second.len(),
                    striker.body().position,
                    cop.body().position,
                    striker.half_dimensions(),
                    cop.half_dimensions(),
                    hit.overlap,
                    hit.normal,
                    -relative.dot(hit.normal)
                );
                return;
            }
        }
    }
    panic!("installed primitives must expose a genuinely approaching contact for the wake witness");
}

#[test]
#[ignore = "needs installed cop geometry/physics/sequences; CPU-only wreck motion"]
fn installed_destroyed_cops_sleep_below_authored_threshold_and_wake_on_real_contact() {
    let (dir, data) = data();
    let collision = floor();
    for name in ["copmidsize", "copgto", "copsport", "copsuv"] {
        let mut renderer = NullBackend::new(crate::settings::test_caps::native());
        let mut world = TrafficWorld::new(cop_road());
        world.load_data(data.database(), None);
        let model = installed_model(&dir, &data, &mut renderer, name);
        assert_eq!(model.model.physics.spec.body.sleep_velocity, 1.0, "installed cop sleep threshold witness");
        world.add_cop_model(model);
        spawn(&mut world, name);
        world.cars[0].hold_for_arrest();
        let mut props = PropWorld::default();
        for _ in 0..180 {
            world.step(focus(), &collision, &mut props, &data.surfaces);
            world.cars[0].sync_contacts();
        }
        assert!(world.cars[0].sims_mut().0.telemetry().wheels_on_ground > 2);
        assert_eq!(world.cars[0].sims_mut().0.body().state, BodyState::Awake);
        assert!(world.cars[0].destroy_by_breaker());
        world.cars[0].sims_mut().0.body_mut().linear_velocity = Vec3::Z * 0.25;
        let mut supported = false;
        for _ in 0..240 {
            world.step(focus(), &collision, &mut props, &data.surfaces);
            world.cars[0].sync_contacts();
            supported |= world.cars[0].sims_mut().0.telemetry().wheels_on_ground > 2;
            let controls = world.cars[0].controls();
            assert_eq!(controls.gas, 0.0);
            assert_eq!(controls.steer, 0.0);
            assert_eq!(controls.handbrake, 0.0);
            assert!(controls.brake == 0.0 || controls.brake == 0.25, "retirement brake remains native");
        }
        assert!(supported);
        let cop = world.cars[0].sims_mut().0;
        assert_eq!(cop.body().state, BodyState::Asleep, "{name}: terminal low-speed creeping must end");
        let stopped = cop.body().position;
        for _ in 0..120 {
            world.step(focus(), &collision, &mut props, &data.surfaces);
            world.cars[0].sync_contacts();
        }
        let cop = world.cars[0].sims_mut().0;
        assert_eq!(cop.body().position, stopped, "sleep remains stable without an external hit");
        eprintln!("preparing installed {name} wake witness");
        let mut striker = CarSim::new(data.car("bmwm3gtr").unwrap(), [0.3; 4]);
        approaching_striker(&mut striker, cop);
        assert_eq!(cop.body().state, BodyState::Asleep, "preparing the striker cannot wake the target");
        assert!(striker.collide_with(cop).is_some(), "the wake witness must be an actual resolved car contact");
        assert_eq!(cop.body().state, BodyState::Awake);
        assert!(cop.body().linear_velocity.length() > 0.0, "sleep is not a frozen wreck override");
        world.cars[0].sync_contacts();
        assert_eq!(world.cars[0].sims_mut().0.body().state, BodyState::Awake);
        world.release(&mut renderer);
    }
}
