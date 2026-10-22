use super::*;

fn pair() -> [CarSim; 2] {
    let mut cars = [sim(), sim()];
    cars[0].body_mut().place(Vec3::ZERO, Quat::IDENTITY);
    cars[0].body_mut().linear_velocity = Vec3::Z * 20.0;
    // A shallow longitudinal overlap makes the minimum-separation normal longitudinal too.
    // At z=3 the larger overlap selected the shorter vertical axis, so forward motion was
    // tangent to that contact and the solver correctly reported no approaching response.
    let distance = cars[0].half_dimensions().z + cars[1].half_dimensions().z - 0.1;
    cars[1].body_mut().place(Vec3::Z * distance, Quat::IDENTITY);
    let contact = blackbox_vehicle::rigid_body::obb_contact(&cars[0].collision_box(), &cars[1].collision_box())
        .expect("the fixture has an actual shallow overlap");
    assert!(contact.normal.dot(Vec3::Z).abs() > 0.99);
    cars
}

#[test]
fn reset_collision_refuses_real_car_contacts_in_both_orders_and_recovers() {
    let [mut first, mut second] = pair();
    assert!(first.collide_with(&mut second).is_some(), "the approaching overlap must actually collide");
    for protected in [0, 1] {
        let mut cars = pair();
        cars[protected].begin_reset_collision_grace();
        let before = cars.each_ref().map(|car| (car.body().position, car.body().linear_velocity));
        let (a, b) = cars.split_at_mut(1);
        assert!(a[0].collide_with(&mut b[0]).is_none());
        assert_eq!(cars.each_ref().map(|car| (car.body().position, car.body().linear_velocity)), before);
        assert!(cars.iter().all(|car| car.pending_sound_contacts.is_empty() && car.pending_effects.is_empty()));
        // The real refused contact refreshes the protected endpoint. Advance on the production
        // fixed clock without moving the fixture, then exercise the same pair again.
        for _ in 0..60 {
            cars[protected].reset_collision.advance(FIXED_STEP);
        }
        assert!(cars[protected].reset_collision_active());
        let (a, b) = cars.split_at_mut(1);
        assert!(a[0].collide_with(&mut b[0]).is_none());
        for _ in 0..150 {
            cars[protected].reset_collision.advance(FIXED_STEP);
        }
        assert!(cars[protected].reset_collision_active(), "a refreshed overlap pauses the remaining timer");
        for _ in 0..60 {
            cars[protected].reset_collision.advance(FIXED_STEP);
        }
        assert!(!cars[protected].reset_collision_active());
        let (a, b) = cars.split_at_mut(1);
        assert!(a[0].collide_with(&mut b[0]).is_some(), "normal response returns after the grace expires");
    }
}

#[test]
fn reset_collision_admission_does_not_refresh_on_a_broad_phase_miss() {
    let [mut first, mut second] = pair();
    first.reset_collision.advance(2.0);
    first.begin_reset_collision_grace();
    second.body_mut().position = Vec3::Z * 100.0;
    assert!(first.collide_with(&mut second).is_none());
    for _ in 0..121 {
        first.reset_collision.advance(FIXED_STEP);
    }
    assert!(!first.reset_collision_active(), "nearby candidates without overlap must not extend immunity");
}

#[test]
fn reset_collision_refreshes_both_active_endpoints_when_the_first_refuses() {
    let mut cars = pair();
    for car in &mut cars {
        car.reset_collision.advance(2.0);
        car.begin_reset_collision_grace();
    }
    let (a, b) = cars.split_at_mut(1);
    assert!(a[0].collide_with(&mut b[0]).is_none());
    for car in &mut cars {
        car.reset_collision.advance(1.0);
        car.reset_collision.advance(1.25);
        assert!(car.reset_collision_active(), "each actual-overlap hook must refresh, despite first refusing");
    }
}

#[test]
fn reset_collision_refuses_resident_prop_response_but_keeps_ground_and_walls() {
    let collision = CollisionWorld::new(None);
    for protected in [false, true] {
        let mut props = PropWorld::default();
        props.add_tile(0, &mut [super::contact_order::attached_pole()]);
        let mut car = sim();
        car.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
        car.body_mut().linear_velocity = Vec3::Z * 30.0;
        if protected {
            car.begin_reset_collision_grace();
        }
        let impact = car.step(&DriveInput::default(), &NoGround, Some((&collision, &mut props)));
        assert_eq!(impact.impulse > 0.0, !protected, "the exact same actual prop overlap uses admission");
        assert_eq!(!impact.sounds.is_empty(), !protected);
        assert_eq!(!impact.damage.is_empty(), !protected);
        assert_eq!(car.take_prop_struck(), !protected);
    }
    let mut car = sim();
    assert!(car.place(&FlatGround::new(0.0), Spawn { position: Vec3::ZERO, heading: 0.0 }));
    for _ in 0..120 {
        car.step(&DriveInput::default(), &FlatGround::new(0.0), None);
    }
    assert!(car.telemetry().wheels_on_ground > 0, "settled positive control must have wheel support");
    car.begin_reset_collision_grace();
    for _ in 0..30 {
        car.step(&DriveInput::default(), &FlatGround::new(0.0), None);
    }
    assert!(car.reset_collision_active());
    assert!(car.telemetry().wheels_on_ground > 0, "wheel support remains enabled");
    car.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
    car.body_mut().linear_velocity = Vec3::Z * 30.0;
    let impact = car.step(
        &DriveInput::default(),
        &NoGround,
        Some((&super::contact_order::nearby_wall(), &mut PropWorld::default())),
    );
    assert!(impact.impulse > 0.0, "world/barrier response remains enabled during reset grace");
}
