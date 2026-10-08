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
