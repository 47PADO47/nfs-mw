use super::*;

#[test]
fn ai_nitrous_capability_uses_the_current_bottle_and_physical_tuning() {
    let mut car = sim();
    let mut spec = car.vehicle.spec().clone();
    spec.nos.nos_capacity = 120.0;
    spec.nos.torque_boost = 1.0;
    car.vehicle.set_spec(spec);
    for (tuning, boost, seconds) in [(-1.0, 0.75, 150.0), (0.0, 1.0, 120.0), (1.0, 1.25, 90.0)] {
        car.vehicle.tunings.nos = tuning;
        let effective = car.nitrous_spec();
        assert_eq!((effective.torque_boost, effective.nos_capacity), (boost, seconds));
        assert!(effective.present());
    }
}
