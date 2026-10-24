//! The requested-speed handoff measured through a stock police car's physical path.

use blackbox_driver::{CarLimits, ControllerKind, DriveFlags, DriveRequest, Driver, SpeedGovernor};
use blackbox_vehicle::{FIXED_STEP, FlatGround};
use glam::Vec3;

use super::CarSim;
use crate::scenes::world::drive::DriveInput;
use crate::scenes::world::road::Spawn;

#[derive(Clone, Copy, Debug)]
enum Mode {
    Throttle,
    Driver,
    MeasuredGovernor,
    AnalyticGovernor,
}

#[test]
#[ignore = "needs installed stock police physics; renderer-free acceleration handoff diagnostic"]
fn installed_cop_acceleration_handoff_exposes_requested_and_physical_speed() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    let physics = data.car("copmidsize").unwrap();
    let performance = nfsmw_data::car::physics::measured::performance_of_spec(&physics.spec, physics.tire_integration);
    let ai = nfsmw_data::pursuit::ai_vehicle(data.database(), "copmidsize");
    let analytical =
        nfsmw_data::car::physics::measured::ai_performance_of_spec(&physics.spec, physics.tire_integration);
    assert_eq!(
        Some(analytical),
        blackbox_vehicle::performance::estimate_ai(&physics.spec),
        "installed police must use the analytical path, not the fallback"
    );
    assert_ne!(analytical.acceleration, performance.acceleration, "estimate and physical caches must remain separate");
    let ground = FlatGround::new(0.0);
    let mut final_speeds = Vec::new();
    for mode in [Mode::Throttle, Mode::Driver, Mode::MeasuredGovernor, Mode::AnalyticGovernor] {
        let performance = match mode {
            Mode::AnalyticGovernor => analytical,
            _ => performance,
        };
        let limits = CarLimits {
            top_speed: performance.top_speed,
            start_grip: performance.start_grip,
            end_grip: performance.end_grip,
            top_speed_multiplier: ai.top_speed_multiplier,
            acceleration_multiplier: ai.acceleration_multiplier,
        };
        let ceiling = (performance.top_speed * ai.top_speed_multiplier * 0.925).min(ai.max_speed_kmh / 3.6);
        assert!(ceiling > 10.0 && performance.acceleration[0] > 0.0, "{performance:?} {ai:?}");
        let mut car = CarSim::new(physics.clone(), [0.3; 4]);
        car.configure_ai();
        car.collision_role.cop = true;
        assert!(car.place_traffic_moving(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }, 0.0));
        let mut driver = Driver::new(ControllerKind::Pid, false);
        let mut governor = SpeedGovernor::begin(0.0);
        let mut wanted = ceiling;
        let mut supported = 0;
        let mut minimum_speed = 0.0_f32;
        for tick in 0..300 {
            let view = car.driver_view();
            if matches!(mode, Mode::MeasuredGovernor | Mode::AnalyticGovernor) && tick % 8 == 0 {
                let acceleration = blackbox_driver::potential_acceleration(
                    performance.acceleration_at(view.forward_speed.abs().max(governor.speed_limit)),
                    &limits,
                    0.5,
                    None,
                    0.0,
                );
                wanted = governor.update(view.forward_speed, ceiling, acceleration, 0.5, 8.0 * FIXED_STEP);
            }
            let mut input = DriveInput { throttle: 1.0, ..Default::default() };
            if !matches!(mode, Mode::Throttle) {
                let request =
                    DriveRequest { target: view.position + Vec3::Z * 100.0, speed: wanted, flags: DriveFlags::FULL };
                let controls = driver.step(&view, &request, FIXED_STEP, tick as f32 * FIXED_STEP);
                input = DriveInput {
                    throttle: controls.gas,
                    brake: controls.brake,
                    steer: controls.steer,
                    handbrake: controls.handbrake > 0.5,
                    ..Default::default()
                };
                assert!(controls.gear.is_none(), "ahead-only fixture must not request reversing: {controls:?}");
            }
            car.step(&input, &ground, None);
            assert!(car.is_finite(), "{mode:?}: tick {tick}");
            let telemetry = car.telemetry();
            minimum_speed = minimum_speed.min(telemetry.speed_mps);
            supported += usize::from(telemetry.wheels_on_ground >= 2);
            if [60, 180, 300].contains(&(tick + 1)) {
                eprintln!(
                    "COP_ACCEL mode={mode:?} seconds={:.1} actual={:.4} requested={wanted:.4} gas={:.3} brake={:.3} gear={} rpm={:.1} wheels={} table={:?}",
                    (tick + 1) as f32 * FIXED_STEP,
                    telemetry.speed_mps,
                    input.throttle,
                    input.brake,
                    telemetry.gear,
                    telemetry.rpm,
                    telemetry.wheels_on_ground,
                    performance.acceleration
                );
            }
        }
        assert!(supported >= 270, "{mode:?}: supported {supported}/300");
        assert!(minimum_speed > -0.5, "{mode:?}: physical car drove backwards {minimum_speed}");
        let actual = car.telemetry().speed_mps;
        assert!(actual > 5.0, "{mode:?}: stock cop failed to accelerate: {actual}");
        final_speeds.push((actual, wanted));
    }
    let available = final_speeds[0].0;
    for (actual, wanted) in &final_speeds[1..] {
        let achievable = available.min(*wanted);
        assert!(
            *actual >= achievable * 0.6,
            "driver/governor stalls: actual={actual} request={wanted} full_throttle={available}"
        );
    }
}
