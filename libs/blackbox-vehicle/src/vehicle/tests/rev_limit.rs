//! The rev limiter (`EngineSpec::no_rev_limiter`): the red line is the limit by default; lifted, the engine
//! runs on along its torque curve up to the last sample. The example car's drag stops it short of its red
//! line in practice, so these tests check the rule in the engine spec.

use super::*;
use crate::engine::EngineSpec;

fn engine(lift: bool) -> EngineSpec {
    let mut spec = VehicleSpec::example().engine;
    spec.no_rev_limiter = lift;
    spec
}

#[test]
fn the_red_line_is_the_limit_by_default() {
    let spec = engine(false);
    assert_eq!(spec.rev_limit(), spec.red_line);
    assert_eq!(spec.torque_ftlb(spec.red_line + 500.0), spec.torque_ftlb(spec.red_line));
}

#[test]
fn lifting_the_limiter_raises_the_limit_to_the_top_of_the_curve() {
    let spec = engine(true);
    assert_eq!(spec.rev_limit(), spec.max_rpm);
    let past = spec.red_line + 500.0;
    assert_eq!(spec.torque_ftlb(past), table_at(&spec, past), "the curve is followed past the red line");
    assert_ne!(spec.torque_ftlb(past), spec.torque_ftlb(spec.red_line), "and it is not the red-line torque");
}

#[test]
fn the_powertrain_follows_the_lifted_limit() {
    let mut v = parked();
    v.set_no_rev_limiter(true);
    assert_eq!(v.powertrain.engine_spec().rev_limit(), VehicleSpec::example().engine.max_rpm);
    v.set_no_rev_limiter(false);
    assert_eq!(v.powertrain.engine_spec().rev_limit(), VehicleSpec::example().engine.red_line);
}

/// The curve's own value at `rpm`, as the spec evaluates it (no clamp).
fn table_at(spec: &EngineSpec, rpm: f32) -> f32 {
    crate::math::table(&spec.torque, spec.idle, spec.max_rpm, rpm)
}
