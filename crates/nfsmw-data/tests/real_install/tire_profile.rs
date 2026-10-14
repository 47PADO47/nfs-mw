//! Installed stock/upgraded racer integration checks; these establish usability, not PC replay parity.

use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, tires::TireIntegration};
use nfsmw_data::car::{
    CarTables,
    physics::{PerformanceCatalog, PerformanceCategory, PerformancePackage, PhysicsData},
};

fn packages(catalog: &PerformanceCatalog) -> [(&'static str, PerformancePackage); 3] {
    let maximum = catalog.maximum();
    let mut junkman = maximum;
    for category in PerformanceCategory::ALL {
        if catalog.junkman_available(category, maximum) {
            junkman.junkman |= 1 << category as u8;
        }
    }
    [("stock", PerformancePackage::default()), ("maximum", maximum), ("maximum + eligible Junkman", junkman)]
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn reference_tire_profile_reverses_installed_racers_and_police_stock_and_upgraded() {
    let dir = crate::install().expect("this check requires NFSMW_GAME_DIR");
    let tables = CarTables::load(&dir).unwrap();
    let physics = PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    let mut checked = 0;
    for folder in nfsmw_data::car::list(&dir) {
        let Some(kind) = tables.car_type_for_folder(&folder) else { continue };
        if kind.usage_type > 1 {
            continue;
        }
        let Ok(stock) = physics.car(&kind.type_name) else { continue };
        if stock.spec.engine.torque.len() < 2
            || !stock.spec.engine.torque.iter().any(|t| *t > 0.0)
            || stock.spec.engine.red_line <= stock.spec.engine.idle
            || stock.spec.transmission.gear_ratio.len() <= 2
            || stock.spec.transmission.gear_ratio[2] <= 0.0
        {
            continue;
        }
        let catalog = PerformanceCatalog::read(physics.database(), &kind.type_name).unwrap();
        for (label, package) in packages(&catalog) {
            let spec = physics.car_with_performance(&kind.type_name, package).unwrap().spec;
            let mut car = Vehicle::new(spec);
            car.set_tire_integration(TireIntegration::Reference);
            assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
            for _ in 0..180 {
                car.step(FIXED_STEP, &InputState::default(), &ground);
            }
            for _ in 0..300 {
                car.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &ground);
            }
            assert_eq!(car.gear(), 0, "{folder} {label}");
            assert!(car.forward_speed() < -0.5, "{folder} {label}: physical reverse {}", car.forward_speed());
            assert!(car.powertrain().speedometer() > 0.5, "{folder} {label}: unsigned reverse speedometer");
            assert!(car.position().is_finite() && car.angular_velocity().is_finite(), "{folder} {label}");
            checked += 1;
        }
    }
    assert_eq!(checked, 144, "all 48 installed playable cars and three package configurations must run");
    println!("{checked} installed reference reverse cases");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn reference_tire_profile_drives_installed_racers_and_police_stock_and_upgraded() {
    let dir = crate::install().expect("this check requires NFSMW_GAME_DIR");
    let tables = CarTables::load(&dir).unwrap();
    let physics = PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    let mut checked = 0;
    let mut junkman_checked = 0;
    let mut failures = Vec::new();
    for folder in nfsmw_data::car::list(&dir) {
        let Some(kind) = tables.car_type_for_folder(&folder) else { continue };
        if kind.usage_type > 1 {
            continue;
        }
        let Ok(stock) = physics.car(&kind.type_name) else { continue };
        if stock.spec.engine.torque.len() < 2
            || !stock.spec.engine.torque.iter().any(|t| *t > 0.0)
            || stock.spec.engine.red_line <= stock.spec.engine.idle
            || stock.spec.transmission.gear_ratio.len() <= 2
            || stock.spec.transmission.gear_ratio[2] <= 0.0
        {
            continue;
        }
        let catalog = PerformanceCatalog::read(physics.database(), &kind.type_name).unwrap();
        for (label, package) in packages(&catalog) {
            if package.junkman != 0 {
                junkman_checked += 1;
            }
            let spec = physics.car_with_performance(&kind.type_name, package).unwrap().spec;
            let mut car = Vehicle::new(spec);
            car.jump_config = nfsmw_data::car::physics::jump_config();
            car.set_tire_integration(TireIntegration::Reference);
            assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
            car.config.auto_reverse = false;
            for _ in 0..180 {
                car.step(FIXED_STEP, &InputState::default(), &ground);
            }
            for _ in 0..1800 {
                // Burnout intentionally biases one wheel. A driver counters that heading change;
                // test road following with bounded steering rather than assuming zero-input yaw.
                let heading = car.rotation().z_axis.x.atan2(car.rotation().z_axis.z);
                // Look at least 30 metres / one second ahead; a fixed distance over-corrects
                // at the police SUV's much higher installed straight-line speed.
                let desired = (-car.position().x / car.speed().max(30.0)).atan();
                let error =
                    (desired - heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                let steer = (4.0 * error - 0.5 * car.angular_velocity().y).clamp(-1.0, 1.0);
                car.step(
                    FIXED_STEP,
                    &InputState { throttle: 1.0, steer, nos: package.junkman != 0, ..Default::default() },
                    &ground,
                );
            }
            let speed = car.forward_speed();
            println!("{folder} {label}: {speed:.2} m/s, gear {}, lateral {:.2} m", car.gear(), car.position().x);
            if !car.position().is_finite() || speed <= 15.0 || car.gear() <= 2 || car.position().x.abs() > 2.0 {
                failures.push(format!(
                    "{folder} {label}: launch/shift/straight-line speed {speed}, gear {}, x {}",
                    car.gear(),
                    car.position().x
                ));
            }
            for _ in 0..600 {
                car.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &ground);
            }
            if !car.linear_velocity().is_finite() || car.speed() > 1.0 {
                failures.push(format!("{folder} {label}: failed braking, speed {}", car.speed()));
            }
            checked += 1;
        }
    }
    assert!(checked >= 105, "only {checked} racer/package checks selected");
    assert!(junkman_checked > 0, "no eligible Junkman driving cases selected");
    println!("{checked} driving cases, including {junkman_checked} cars with Junkman hardware");
    assert!(failures.is_empty(), "{} failures out of {checked}:\n{}", failures.len(), failures.join("\n"));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn reference_tire_profile_mirrors_installed_cornering_stock_and_upgraded() {
    let dir = crate::install().expect("this check requires NFSMW_GAME_DIR");
    let tables = CarTables::load(&dir).unwrap();
    let physics = PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    let mut checked = 0;
    let mut junkman_checked = 0;
    for folder in nfsmw_data::car::list(&dir) {
        let Some(kind) = tables.car_type_for_folder(&folder) else { continue };
        if kind.usage_type > 1 {
            continue;
        }
        let Ok(stock) = physics.car(&kind.type_name) else { continue };
        if stock.spec.engine.torque.len() < 2
            || !stock.spec.engine.torque.iter().any(|t| *t > 0.0)
            || stock.spec.engine.red_line <= stock.spec.engine.idle
            || stock.spec.transmission.gear_ratio.len() <= 2
            || stock.spec.transmission.gear_ratio[2] <= 0.0
        {
            continue;
        }
        let catalog = PerformanceCatalog::read(physics.database(), &kind.type_name).unwrap();
        for (label, package) in packages(&catalog) {
            if package.junkman != 0 {
                junkman_checked += 1;
            }
            let mut left = Vehicle::new(physics.car_with_performance(&kind.type_name, package).unwrap().spec);
            left.jump_config = nfsmw_data::car::physics::jump_config();
            left.set_tire_integration(TireIntegration::Reference);
            left.place_on_ground_moving(&ground, 0.0, 0.0, 5.0, 0.0, 20.0);
            for _ in 0..60 {
                left.step(FIXED_STEP, &InputState { throttle: 0.4, ..Default::default() }, &ground);
            }
            let mut right = left.clone();
            let mut yaw = [0.0_f32; 2];
            for _ in 0..240 {
                left.step(
                    FIXED_STEP,
                    &InputState { throttle: 0.4, steer: -0.35, nos: package.junkman != 0, ..Default::default() },
                    &ground,
                );
                right.step(
                    FIXED_STEP,
                    &InputState { throttle: 0.4, steer: 0.35, nos: package.junkman != 0, ..Default::default() },
                    &ground,
                );
                assert!(left.position().is_finite() && right.position().is_finite(), "{folder} {label}");
                yaw[0] = yaw[0].max(-left.angular_velocity().y);
                yaw[1] = yaw[1].max(right.angular_velocity().y);
            }
            assert!(yaw[0] > 0.01 && yaw[1] > 0.01, "{folder} {label}: no cornering response {yaw:?}");
            assert!((left.position().x + right.position().x).abs() < 0.2, "{folder} {label}: asymmetric x");
            assert!((left.position().z - right.position().z).abs() < 0.2, "{folder} {label}: asymmetric z");
            assert!((left.speed() - right.speed()).abs() < 0.05, "{folder} {label}: asymmetric speed");
            checked += 1;
        }
    }
    assert!(checked >= 105, "only {checked} cornering/package checks selected");
    assert!(junkman_checked > 0, "no eligible Junkman cornering cases selected");
    println!("{checked} cornering cases, including {junkman_checked} cars with Junkman hardware");
}

fn finite_dynamics(car: &Vehicle) -> bool {
    car.position().is_finite()
        && car.orientation().is_finite()
        && car.linear_velocity().is_finite()
        && car.angular_velocity().is_finite()
        && car.rpm().is_finite()
        && car.rpm() >= 0.0
        && car.speed() < 200.0
        && (0..4).all(|i| {
            let wheel = car.wheel(i);
            wheel.position.is_finite()
                && [
                    wheel.radius,
                    wheel.steer_angle,
                    wheel.angular_velocity,
                    wheel.simulated_angular_velocity,
                    wheel.compression,
                    wheel.load,
                    wheel.slip,
                    wheel.slip_angle,
                    wheel.traction,
                    wheel.lateral_force,
                    wheel.longitudinal_force,
                ]
                .into_iter()
                .all(f32::is_finite)
        })
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn original_tuning_extremes_drive_all_installed_racers_and_police() {
    use nfsmw_data::car::physics::tuning::{TuningCategory, TuningSelection};

    let dir = crate::install().expect("this check requires NFSMW_GAME_DIR");
    let tables = CarTables::load(&dir).unwrap();
    let physics = PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    let mut checked = 0;
    let mut cars = 0;
    let mut tunable_cars = 0;
    let mut junkman_checked = 0;
    let mut failures = Vec::new();
    for folder in nfsmw_data::car::list(&dir) {
        let Some(kind) = tables.car_type_for_folder(&folder) else { continue };
        if kind.usage_type > 1 {
            continue;
        }
        let Ok(stock) = physics.car(&kind.type_name) else { continue };
        if stock.spec.engine.torque.len() < 2
            || !stock.spec.engine.torque.iter().any(|t| *t > 0.0)
            || stock.spec.engine.red_line <= stock.spec.engine.idle
            || stock.spec.transmission.gear_ratio.len() <= 2
            || stock.spec.transmission.gear_ratio[2] <= 0.0
        {
            continue;
        }
        cars += 1;
        let catalog = PerformanceCatalog::read(physics.database(), &kind.type_name).unwrap();
        let available = TuningCategory::ALL.map(|category| category.available(catalog.maximum(), false));
        if folder == "COPSUV" || folder == "COPSUVL" {
            println!("{folder}: installed maxima {:?}, reachable tuning {available:?}", catalog.maximum().levels);
        }
        if !available.into_iter().any(|value| value) {
            println!("{folder}: no reachable tuning; neutral driving covered by stock/upgraded baseline suite");
            continue;
        }
        tunable_cars += 1;
        // Start with values reachable through this car's ordinary hardware upgrades. Retain them
        // when returning to stock as the original does. The overlay has no aftermarket aero parts.
        for (label, package) in packages(&catalog).into_iter().filter(|(label, _)| *label != "maximum") {
            for step in [-5, 5] {
                let spec = physics.car_with_performance(&kind.type_name, package).unwrap().spec;
                let mut car = Vehicle::new(spec);
                car.set_tire_integration(TireIntegration::Reference);
                car.jump_config = nfsmw_data::car::physics::jump_config();
                let profile = available.map(|enabled| match enabled {
                    true => step,
                    false => 0,
                });
                car.tunings = TuningSelection { profiles: [profile; 3], active: 0 }.normalized().unwrap();
                car.config.auto_reverse = false;
                assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
                let mut finite = true;
                for _ in 0..180 {
                    car.step(FIXED_STEP, &InputState::default(), &ground);
                    finite &= finite_dynamics(&car);
                }
                for _ in 0..1800 {
                    let heading = car.rotation().z_axis.x.atan2(car.rotation().z_axis.z);
                    let desired = (-car.position().x / car.speed().max(30.0)).atan();
                    let error = (desired - heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    let steer = (4.0 * error - 0.5 * car.angular_velocity().y).clamp(-1.0, 1.0);
                    car.step(
                        FIXED_STEP,
                        &InputState { throttle: 1.0, steer, nos: true, ..Default::default() },
                        &ground,
                    );
                    finite &= finite_dynamics(&car);
                }
                let speed = car.forward_speed();
                if !finite || speed <= 15.0 || car.gear() <= 2 || car.position().x.abs() > 2.0 {
                    failures.push(format!(
                        "{folder} {label} tuning {step}: finite={finite}, launch={speed}, gear={}, x={}",
                        car.gear(),
                        car.position().x
                    ));
                }
                for _ in 0..600 {
                    car.step(FIXED_STEP, &InputState { brake: 1.0, ..Default::default() }, &ground);
                    finite &= finite_dynamics(&car);
                }
                if !finite || car.speed() > 1.0 {
                    failures.push(format!("{folder} {label} tuning {step}: finite={finite}, braking={}", car.speed()));
                }
                checked += 1;
                junkman_checked += usize::from(package.junkman != 0);
            }
        }
    }
    assert_eq!(cars, 48, "playable catalog coverage changed");
    assert!(tunable_cars >= 20, "only {tunable_cars} tunable cars selected");
    assert_eq!(checked, tunable_cars * 4);
    assert!(junkman_checked > 0, "no eligible Junkman cases selected");
    println!(
        "{checked} reachable tuning-extreme drive/brake cases across {tunable_cars}/{cars} cars, {junkman_checked} with Junkman"
    );
    assert!(failures.is_empty(), "{} failures out of {checked}:\n{}", failures.len(), failures.join("\n"));
}
