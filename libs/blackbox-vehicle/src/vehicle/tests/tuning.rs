//! Retained profile values reach tire/brake and suspension consumers after hardware replacement.

use super::*;

#[test]
fn retained_tunings_still_control_handling_brakes_and_ride_height() {
    for (slider, front, rear, lateral, longitudinal) in
        [(-1.0, 0.5, 1.5, 0.8, 1.2), (0.0, 1.0, 1.0, 1.0, 1.0), (1.0, 1.5, 0.5, 1.2, 0.8)]
    {
        let spec = VehicleSpec::example();
        let base_ride = spec.chassis.axle(0, 0.0).ride;
        let mut car = Vehicle::new(spec.clone());
        car.tunings = Tunings { handling: slider, brakes: slider, ride_height: slider, ..Default::default() };
        car.set_spec(spec);
        car.controls.brake = 1.0;
        car.tune_tires(FIXED_STEP, 30.0, car.body.angular_velocity);
        assert!((car.tires[0].brake - front).abs() < 1e-6);
        assert!((car.tires[2].brake - rear).abs() < 1e-6);
        for tire in &car.tires {
            assert!((tire.traction_circle.0 - lateral).abs() < 1e-6);
            assert!((tire.traction_circle.1 - longitudinal).abs() < 1e-6);
        }
        let tuned_ride = car.spec.chassis.axle(0, car.tunings.ride_height).ride;
        assert!((tuned_ride - base_ride - slider * 0.0254).abs() < 1e-6);
    }
}
