//! Installed-data coverage at explicit deltas; this is not original-game trajectory acceptance.

use blackbox_vehicle::{FlatGround, InputState, Vehicle};
use nfsmw_data::car::physics::{
    CarPhysics, PerformanceCatalog, PerformanceCategory, PerformancePackage, PhysicsData,
    tuning::{TuningCategory, TuningSelection},
};

fn packages(catalog: &PerformanceCatalog) -> Vec<PerformancePackage> {
    let maximum = catalog.maximum();
    let mut junkman = maximum;
    for category in PerformanceCategory::ALL {
        if catalog.junkman_available(category, maximum) {
            junkman.junkman |= 1 << category as u8;
        }
    }
    let mut packages = Vec::new();
    for package in [PerformancePackage::default(), maximum, junkman] {
        if !packages.contains(&package) {
            packages.push(package);
        }
    }
    packages
}

fn profiles(package: PerformancePackage) -> Vec<[i8; 7]> {
    let mut profiles = vec![[0; 7]];
    for category in TuningCategory::ALL {
        // These fixtures have no aftermarket aero; hardware gates the other six sliders.
        if !category.available(package, false) {
            continue;
        }
        for endpoint in [-5, 5] {
            let mut profile = [0; 7];
            profile[category as usize] = endpoint;
            profiles.push(profile);
        }
    }
    profiles
}

fn vehicle(physics: &CarPhysics, profile: [i8; 7], ground: &FlatGround) -> Vehicle {
    let mut car = Vehicle::new(physics.spec.clone());
    car.set_tire_integration(physics.tire_integration);
    car.jump_config = physics.jump_config;
    car.wall_steering_config = nfsmw_data::car::physics::wall_steering_config();
    car.tunings = TuningSelection { profiles: [profile; 3], active: 0 }.normalized().unwrap();
    car.config.auto_reverse = false;
    assert!(car.place_on_ground(ground, 0.0, 0.0, 5.0, 0.0));
    car
}

fn step_for(car: &mut Vehicle, ground: &FlatGround, dt: f32, seconds: f32, input: InputState, context: &str) {
    let steps = (seconds / dt).round() as usize;
    assert!(steps > 0);
    for tick in 0..steps {
        car.step(dt, &input, ground);
        let finite = car.position().is_finite()
            && car.orientation().is_finite()
            && car.linear_velocity().is_finite()
            && car.angular_velocity().is_finite()
            && car.rpm().is_finite()
            && (0..4).all(|index| {
                let wheel = car.wheel(index);
                [
                    wheel.load,
                    wheel.slip_angle,
                    wheel.traction,
                    wheel.lateral_force,
                    wheel.longitudinal_force,
                    wheel.simulated_angular_velocity,
                ]
                .into_iter()
                .all(f32::is_finite)
            });
        assert!(finite, "{context}, tick {tick}, input {input:?}");
        assert!(
            car.rpm() >= 0.0 && car.rpm() <= car.spec().engine.max_rpm,
            "{context}: RPM {} outside installed engine range",
            car.rpm()
        );
        assert!(car.gear() <= car.spec().transmission.top_gear(), "{context}: invalid gear {}", car.gear());
    }
}

#[test]
#[ignore = "requires a PC installation; explicit driving deltas, not original-game parity"]
fn installed_player_hardware_and_tuning_matrix_handles_normal_and_quarter_speed_deltas() {
    let dir = crate::install().expect("this check requires NFSMW_GAME_DIR");
    let data = PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    let mut cases = 0;
    let mut package_count = 0;
    let mut tuning_count = 0;
    let mut junkman_cases = 0;
    let mut drivetrains = [false; 3];
    for name in ["gti", "bmwm3gtr", "911turbo"] {
        let catalog = PerformanceCatalog::read(data.database(), name).unwrap();
        let available = packages(&catalog);
        package_count += available.len();
        for package in available {
            let physics = data.car_with_performance(name, package).unwrap();
            let front = physics.spec.transmission.front_driven();
            let rear = physics.spec.transmission.rear_driven();
            let drivetrain = match (front, rear) {
                (true, false) => 0,
                (false, true) => 1,
                (true, true) => 2,
                _ => panic!("{name}: no driven axle"),
            };
            drivetrains[drivetrain] = true;
            let profiles = profiles(package);
            tuning_count += profiles.len();
            for profile in profiles {
                for rate in [60, 144] {
                    for scale in [1.0, 0.25] {
                        let dt = scale / rate as f32;
                        let context = format!("{name}, {package:?}, tuning {profile:?}, {rate} Hz, scale {scale}");
                        let mut car = vehicle(&physics, profile, &ground);
                        step_for(&mut car, &ground, dt, 0.25, InputState::default(), &context);
                        let rest_position = car.position();
                        step_for(
                            &mut car,
                            &ground,
                            dt,
                            1.5,
                            InputState { throttle: 1.0, nos: true, ..Default::default() },
                            &context,
                        );
                        assert!(
                            car.forward_speed() > 0.0 && car.position().distance(rest_position) > 0.0,
                            "{context}: did not accelerate from rest"
                        );
                        step_for(
                            &mut car,
                            &ground,
                            dt,
                            0.25,
                            InputState { throttle: 1.0, brake: 1.0, ..Default::default() },
                            &context,
                        );
                        step_for(&mut car, &ground, dt, 0.25, InputState::default(), &context);
                        step_for(
                            &mut car,
                            &ground,
                            dt,
                            0.25,
                            InputState { throttle: 0.5, steer: 0.25, ..Default::default() },
                            &context,
                        );
                        // Establish forward motion before isolating the brake response.
                        step_for(
                            &mut car,
                            &ground,
                            dt,
                            0.75,
                            InputState { throttle: 1.0, ..Default::default() },
                            &context,
                        );
                        let before_brake = car.speed();
                        assert!(before_brake > 0.0, "{context}: no motion before braking");
                        step_for(&mut car, &ground, dt, 1.0, InputState { brake: 1.0, ..Default::default() }, &context);
                        assert!(car.speed() < before_brake, "{context}: braking did not reduce speed");
                        cases += 1;
                        junkman_cases += usize::from(package.junkman != 0);
                    }
                }
            }
        }
    }
    assert_eq!(drivetrains, [true; 3], "FWD, RWD and AWD must all run");
    assert!(junkman_cases > 0 && tuning_count > package_count, "hardware/tuning coverage was empty");
    assert_eq!(cases, tuning_count * 4);
    println!(
        "{cases} directed driving cases: 3 drivetrains, {package_count} unique car/package pairs, \
              {tuning_count} reachable profiles, {junkman_cases} Junkman cases; 60/144 Hz normal/quarter dt. \
              Finite-state and acceleration/braking usability only; no OG trajectory/countdown acceptance."
    );
}
