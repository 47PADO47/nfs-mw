//! A shallower approaching body part must survive an earlier nonclosing overlap.
use super::*;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shape_at};
use blackbox_collision::{BodyGeometry, BodyPrimitive, BoundsSet, Shape, bounds_flags};

fn primitive(centre: Vec3, half: Vec3, surface: u32, enabled: bool) -> BodyPrimitive {
    BodyPrimitive {
        name: surface,
        centre,
        rotation: Quat::IDENTITY,
        half,
        shape: Shape::Box,
        flags: bounds_flags::PRIM_VS_OBJECTS
            | match enabled {
                true => 0,
                false => bounds_flags::DISABLED,
            },
        surface,
    }
}

fn install(car: &mut CarSim, primitives: Vec<BodyPrimitive>) {
    car.geometry = Some(Arc::new(CarGeometry {
        bounds: Arc::new(BoundsSet { name_hash: 0, nodes: vec![], point_clouds: vec![] }),
        body: BodyGeometry { primitives, ..Default::default() },
    }));
}

#[test]
fn compound_vehicle_dispatch_keeps_the_shallower_approaching_part_after_separation() {
    for enabled in [false, true] {
        let (mut a, mut b) = (sim(), sim());
        a.configure_ai();
        b.configure_ai();
        install(
            &mut a,
            vec![
                primitive(Vec3::new(0.0, -1.25, 0.0), Vec3::new(3.0, 1.0, 3.0), 11, true),
                primitive(Vec3::new(1.5, 0.75, 0.0), Vec3::ONE, 12, enabled),
            ],
        );
        install(&mut b, vec![primitive(Vec3::ZERO, Vec3::ONE, 21, true)]);
        let mut spec = b.vehicle.spec().clone();
        spec.body.objects.immobile = true;
        b.vehicle.set_spec(spec);
        a.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
        b.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
        a.body_mut().linear_velocity = Vec3::NEG_X * 20.0;
        a.previous_contact_pose = (Vec3::X / 3.0, Quat::IDENTITY);
        b.previous_contact_pose = (Vec3::ZERO, Quat::IDENTITY);
        let frame =
            BodyGeometryFrame { geometry: &a.geometry.as_ref().unwrap().body, previous: a.previous_contact_pose };
        let target_frame =
            BodyGeometryFrame { geometry: &b.geometry.as_ref().unwrap().body, previous: b.previous_contact_pose };
        let target = object_shape_at(b.body(), Some(target_frame), 0).unwrap();
        let lower = object_shape_at(a.body(), Some(frame), 0).unwrap().contact(&target).unwrap();
        assert_eq!(lower.normal, Vec3::NEG_Y);
        assert!((lower.overlap - 0.75).abs() < 0.00001);
        assert_eq!(a.body().linear_velocity.dot(lower.normal), 0.0);
        let (mass, inertia) = (a.body().mass(), a.body().inertia());
        let mut hits = Vec::new();
        let reacted = a.collide_with_each(&mut b, |hit, _, _| hits.push(hit));
        assert_eq!(reacted, enabled, "the nonclosing deeper contact cannot suppress the later part");
        assert_eq!(hits.len(), usize::from(enabled));
        assert!((a.body().position.y + 0.75).abs() < 0.00001, "even the nonclosing contact separates");
        assert_eq!((a.body().mass(), a.body().inertia()), (mass, inertia));
        if enabled {
            assert!(hits[0].impulse > 0.0);
            assert_eq!(a.pending_sound_contacts.len(), 1);
            assert_eq!(b.pending_sound_contacts.len(), 1);
            assert_eq!(b.pending_sound_contacts[0].surface, 12, "callback belongs to the later actual part");
        }
    }
}

#[test]
fn compound_geometry_preparation_retains_a_snapshot_and_refreshes_b_from_its_body() {
    let mut car = sim();
    install(&mut car, vec![primitive(Vec3::X, Vec3::ONE, 7, true)]);
    car.body_mut().place(Vec3::Y * 3.0, Quat::IDENTITY);
    let geometry = car.geometry.clone().unwrap();
    let frame =
        Some(BodyGeometryFrame { geometry: &geometry.body, previous: (car.body().position, car.body().orientation) });
    let prepared = object_shape_at(car.body(), frame, 0).unwrap();
    car.body_mut().position += Vec3::X * 10.0;
    let fresh = object_shape_at(car.body(), frame, 0).unwrap();
    assert_eq!(prepared.bounds.centre, Vec3::new(1.0, 3.0, 0.0));
    assert_eq!(fresh.bounds.centre, Vec3::new(11.0, 3.0, 0.0));
    assert_eq!(prepared.travel, Vec3::ZERO);
    assert_eq!(fresh.travel, Vec3::X * 10.0);
}
