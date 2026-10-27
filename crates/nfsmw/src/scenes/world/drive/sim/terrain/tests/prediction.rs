use super::*;

#[test]
fn prediction_restore_keeps_retained_face_history_under_overlapping_roads() {
    let mut world = ground_face();
    let mut pack = world.remove(1).unwrap();
    let mut upper = pack.articles[0].strips[0].clone();
    upper.center[1] = 5.0;
    assert_eq!(upper.point(0)[1], 5.0, "the strip origin sets the upper road height");
    pack.articles[0].strips.insert(0, upper);
    pack.instances[0].half_height = 8.0;
    world.insert(pack);
    let mut props = PropWorld::default();
    let mut car = CarSim::new(physics(), [0.2; 4]);
    car.vehicle.place(Vec3::new(0.0, 0.49, 0.0), Quat::IDENTITY);
    car.step(&DriveInput::default(), &NoGround, Some((&world, &mut props)));
    assert!(car.ground_reference.plane().valid);
    assert!(car.wheel_ground.borrow().iter().all(|cache| cache.plane().valid));

    // Retain a lower selected face while moving above the upper road. A cold
    // search selects the upper road instead, so discarding this history changes forces.
    car.body_mut().position.y = 5.49;
    let saved = car.save();
    let before = car.ground_reference.plane();
    let wheel_before = car.wheel_ground.borrow().clone().map(|cache| cache.plane());
    let mut run = |car: &mut CarSim| {
        car.step(&DriveInput::default(), &NoGround, Some((&world, &mut props)));
        (
            car.motion(),
            car.body().pending_force(),
            car.body().pending_torque(),
            car.ground_reference.plane(),
            car.vehicle.wheels_on_ground(),
        )
    };
    let expected = run(&mut car);
    car.restore(&saved);
    assert_eq!(car.ground_reference.plane(), before);
    assert_eq!(car.wheel_ground.borrow().clone().map(|cache| cache.plane()), wheel_before);
    assert_eq!(run(&mut car), expected, "restore must reproduce the next completed tick exactly");

    car.restore(&saved);
    car.ground_reference.clear();
    for wheel in car.wheel_ground.get_mut() {
        wheel.clear();
    }
    let cold = run(&mut car);
    assert_ne!(cold.3, expected.3, "control: a fresh selection must choose a different road plane");
    assert_ne!(cold.1, expected.1, "control: lost face history must change physical response");
}
