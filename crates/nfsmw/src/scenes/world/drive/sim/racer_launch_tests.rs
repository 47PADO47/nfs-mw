//! A flat-ground, full-throttle standing start of quick-race opponents on maximum ordinary hardware, through the
//! racer's physical path (`configure_racer`), for comparing the car model with a race trace.

use blackbox_vehicle::FlatGround;
use glam::Vec3;

use super::CarSim;
use crate::scenes::world::drive::DriveInput;
use crate::scenes::world::road::Spawn;

#[derive(Clone, Copy, Debug)]
enum Setup {
    /// The racer chassis' own tire on the simple chassis' contact state (the set-up before the racer change).
    RacerTire,
    /// What a race opponent gets: the simple suspension with its own tires.
    Racer,
}

const MARKS: [f32; 8] = [0.5, 1.0, 2.0, 3.0, 5.0, 8.0, 10.0, 15.0];

#[test]
#[ignore = "needs the installed game; prints a full-throttle launch of each 16.1.1 opponent on maximum hardware"]
fn installed_racer_launch_on_maximum_hardware() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    let ground = FlatGround::new(0.0);
    for car_type in ["gto", "imprezawrx", "rx8"] {
        let package = nfsmw_data::car::physics::PerformanceCatalog::read(data.database(), car_type).unwrap().maximum();
        let physics = data.car_with_performance(car_type, package).unwrap();
        let s = &physics.spec;
        eprintln!(
            "SPEC {car_type} package={:?} mass={} torque={:?} idle={} red_line={} max_rpm={} gears={:?} final={} split={} \
             efficiency={:?} shift_speed={} clutch_slip={} converter={} static_grip={:?} dynamic_grip={:?} \
             grip_scale={:?} radius_front={} radius_rear={} induction=({}, {}) drag={} aero={}",
            package.levels,
            s.mass,
            s.engine.torque,
            s.engine.idle,
            s.engine.red_line,
            s.engine.max_rpm,
            s.transmission.gear_ratio,
            s.transmission.final_gear,
            s.transmission.torque_split,
            s.transmission.gear_efficiency,
            s.transmission.shift_speed,
            s.transmission.clutch_slip,
            s.transmission.torque_converter,
            s.tires.static_grip,
            s.tires.dynamic_grip,
            s.tires.grip_scale,
            s.tires.wheel_diameter(0) * 0.5,
            s.tires.wheel_diameter(1) * 0.5,
            s.induction.low_boost,
            s.induction.high_boost,
            s.aero.drag_coefficient,
            s.aero.aero_coefficient,
        );
        for setup in [Setup::RacerTire, Setup::Racer] {
            let mut car = CarSim::new(physics.clone(), [0.3; 4]);
            match setup {
                Setup::RacerTire => car.configure_ai(),
                Setup::Racer => car.configure_racer(),
            }
            car.collision_racer = true;
            assert!(car.place_moving(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }, 0.0));
            let input = DriveInput { throttle: 1.0, ..Default::default() };
            let mut tick = 0;
            let mut distance = 0.0;
            let mut first_290 = None;
            for mark in MARKS {
                while (tick as f32) < mark * 60.0 - 0.5 {
                    car.step(&input, &ground, None);
                    tick += 1;
                    distance += car.telemetry().speed_mps / 60.0;
                    if distance >= 290.0 && first_290.is_none() {
                        first_290 = Some(tick as f32 / 60.0);
                    }
                }
                let t = car.telemetry();
                let rear = car.vehicle.wheel(2);
                eprintln!(
                    "LAUNCH {car_type} {setup:?} t={mark:>4.1} v={:6.2} gear={} rpm={:6.0} shifting={} rear_slip={:6.2} \
                     rear_traction={:.2} rear_fx={:7.0} wheels={}",
                    t.speed_mps,
                    t.gear,
                    t.rpm,
                    t.shifting,
                    rear.slip,
                    rear.traction,
                    rear.longitudinal_force,
                    t.wheels_on_ground
                );
            }
            while first_290.is_none() && tick < 60 * 30 {
                car.step(&input, &ground, None);
                tick += 1;
                distance += car.telemetry().speed_mps / 60.0;
                if distance >= 290.0 {
                    first_290 = Some(tick as f32 / 60.0);
                }
            }
            eprintln!("LAUNCH {car_type} {setup:?} 290m={first_290:?}");
            assert!(car.telemetry().speed_mps > 20.0, "{car_type} {setup:?} did not launch");
        }
    }
}
