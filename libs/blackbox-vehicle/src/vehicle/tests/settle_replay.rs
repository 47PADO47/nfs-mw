//! Optional numeric boundary replay; fixture data is supplied externally.

use super::*;
use crate::tires::TireIntegration;

fn number(input: &mut std::str::SplitWhitespace<'_>) -> f32 {
    input.next().expect("missing numeric input").parse().expect("invalid numeric input")
}

fn vector(input: &mut std::str::SplitWhitespace<'_>) -> Vec3 {
    Vec3::new(number(input), number(input), number(input))
}

/// Per row: label, reference, grounded count, brake, gas, collision, basis columns,
/// current velocity/angular velocity, angular snapshot, pending force/torque,
/// expected velocity/angular velocity/force/torque. Requires exact f32 outputs.
#[test]
#[ignore = "requires NFSMW_CREEP_REPLAY numeric boundary fixture"]
fn external_numeric_creep_boundaries() {
    let path = std::env::var_os("NFSMW_CREEP_REPLAY").expect("set NFSMW_CREEP_REPLAY");
    let fixture = std::fs::read_to_string(path).expect("read numeric fixture");
    let mut count = 0;
    for line in fixture.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let mut input = line.split_whitespace();
        let label = input.next().expect("row label");
        let integration = match number(&mut input) != 0.0 {
            true => TireIntegration::Reference,
            false => TireIntegration::Regularized,
        };
        let grounded = number(&mut input) as usize;
        let brake = number(&mut input);
        let gas = number(&mut input);
        let collided = number(&mut input) != 0.0;
        let rotation = glam::Mat3::from_cols(vector(&mut input), vector(&mut input), vector(&mut input));
        let mut car = Vehicle::new(VehicleSpec::example());
        car.set_tire_integration(integration);
        car.body.seed_test_rotation(rotation);
        car.wheels_on_ground = grounded;
        car.controls.brake = brake;
        car.controls.gas = gas;
        car.body.linear_velocity = vector(&mut input);
        car.body.angular_velocity = vector(&mut input);
        let angular = vector(&mut input);
        car.body.apply_force(vector(&mut input));
        car.body.apply_torque(vector(&mut input));
        let expected = [vector(&mut input), vector(&mut input), vector(&mut input), vector(&mut input)];
        assert!(input.next().is_none(), "extra input in {label}");
        car.damp_creep(collided, angular);
        let actual =
            [car.body.linear_velocity, car.body.angular_velocity, car.body.pending_force(), car.body.pending_torque()];
        print!("creep_replay {label}");
        for value in actual {
            print!(" {} {} {}", value.x, value.y, value.z);
        }
        println!();
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_eq!(actual, expected, "{label}");
        }
        count += 1;
    }
    assert!(count > 0, "fixture must contain selected boundaries");
    println!("creep_replay_rows {count}");
}
