//! The engine, transmission, induction and nitrous classes of a car, as the vehicle library wants them
//! (`docs/specs/vehicle-engine-drivetrain.md` and `vehicle-input-induction-brakes.md`, "fields read").

use blackbox_vehicle::drivetrain::TransmissionSpec;
use blackbox_vehicle::engine::EngineSpec;
use blackbox_vehicle::induction::InductionSpec;
use blackbox_vehicle::nos::NosSpec;

use super::fields::Fields;

pub fn engine(c: Fields<'_>) -> EngineSpec {
    EngineSpec {
        torque: c.floats("TORQUE"),
        idle: c.f32("IDLE"),
        red_line: c.f32("RED_LINE"),
        max_rpm: c.f32("MAX_RPM"),
        flywheel_mass: c.f32("FLYWHEEL_MASS"),
        engine_braking: c.floats("ENGINE_BRAKING"),
        speed_limiter: c.pair("SPEED_LIMITER"),
    }
}

pub fn transmission(c: Fields<'_>) -> TransmissionSpec {
    let differential = c.floats("DIFFERENTIAL");
    TransmissionSpec {
        gear_ratio: c.floats("GEAR_RATIO"),
        gear_efficiency: c.floats("GEAR_EFFICIENCY"),
        final_gear: c.f32("FINAL_GEAR"),
        torque_split: c.f32("TORQUE_SPLIT"),
        differential: std::array::from_fn(|i| differential.get(i).copied().unwrap_or(0.0)),
        torque_converter: c.f32("TORQUE_CONVERTER"),
        clutch_slip: c.f32("CLUTCH_SLIP"),
        shift_speed: c.f32("SHIFT_SPEED"),
        optimal_shift: c.f32("OPTIMAL_SHIFT"),
    }
}

pub fn induction(c: Fields<'_>) -> InductionSpec {
    InductionSpec {
        low_boost: c.f32("LOW_BOOST"),
        high_boost: c.f32("HIGH_BOOST"),
        spool: c.f32("SPOOL"),
        spool_time_up: c.f32("SPOOL_TIME_UP"),
        spool_time_down: c.f32("SPOOL_TIME_DOWN"),
        vacuum: c.f32("VACUUM"),
        psi: c.f32("PSI"),
    }
}

pub fn nos(c: Fields<'_>) -> NosSpec {
    NosSpec {
        nos_capacity: c.f32("NOS_CAPACITY"),
        torque_boost: c.f32("TORQUE_BOOST"),
        nos_disengage: c.f32("NOS_DISENGAGE"),
        recharge_min: c.f32("RECHARGE_MIN"),
        recharge_max: c.f32("RECHARGE_MAX"),
        recharge_min_speed: c.f32("RECHARGE_MIN_SPEED"),
        recharge_max_speed: c.f32("RECHARGE_MAX_SPEED"),
    }
}

/// Idle and red line (rpm) of the engine a trailer does not have: any positive pair keeps the powertrain's
/// maths well defined, the torque table is empty so it never makes a newton metre.
const NO_ENGINE_IDLE: f32 = 1000.0;
const NO_ENGINE_RED_LINE: f32 = 6000.0;
/// Gears of the transmission a trailer does not have: reverse, neutral, first, all with ratio 0 so no
/// torque ever reaches the wheels.
const NO_TRANSMISSION_GEARS: usize = 3;

/// The engine of a body without one (a trailer): no torque at any speed.
pub fn no_engine() -> EngineSpec {
    EngineSpec {
        torque: Vec::new(),
        idle: NO_ENGINE_IDLE,
        red_line: NO_ENGINE_RED_LINE,
        max_rpm: NO_ENGINE_RED_LINE,
        flywheel_mass: 0.0,
        engine_braking: Vec::new(),
        speed_limiter: [0.0; 2],
    }
}

/// The transmission of a body without one (a trailer): every gear has ratio 0, so the wheels roll free.
pub fn no_transmission() -> TransmissionSpec {
    TransmissionSpec {
        gear_ratio: vec![0.0; NO_TRANSMISSION_GEARS],
        gear_efficiency: vec![1.0; NO_TRANSMISSION_GEARS],
        final_gear: 1.0,
        torque_split: 0.0,
        differential: [0.0; 3],
        torque_converter: 0.0,
        clutch_slip: 0.0,
        shift_speed: 0.0,
        optimal_shift: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, VehicleSpec};

    use super::*;

    #[test]
    fn a_body_without_powertrain_rolls_free_and_brakes() {
        let ground = FlatGround::new(0.0);
        let mut spec = VehicleSpec::example();
        spec.engine = no_engine();
        spec.transmission = no_transmission();
        let mut body = Vehicle::new(spec);
        body.place_on_ground_moving(&ground, 0.0, 0.0, 5.0, 0.0, 10.0);
        let full_throttle = InputState { throttle: 1.0, ..InputState::default() };
        for _ in 0..120 {
            body.step(FIXED_STEP, &full_throttle, &ground);
        }
        let coasting = body.forward_speed();
        assert!(coasting > 8.0 && coasting <= 10.0, "no engine drives the body: {coasting} m/s");
        let brake = InputState { brake: 1.0, ..InputState::default() };
        for _ in 0..240 {
            body.step(FIXED_STEP, &brake, &ground);
        }
        assert!(body.forward_speed().abs() < coasting - 5.0, "the brakes act: {} m/s", body.forward_speed());
        assert!(body.position().is_finite());
    }
}
