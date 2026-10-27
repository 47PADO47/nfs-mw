//! Optional measurement adapter. External numeric values never ship with it.

use super::*;
use crate::vehicle::{
    AlignmentConfig, GroundReference, JumpConfig, JumpInput, JumpState, LandingConfig, PassiveConfig,
};

fn number(input: &mut std::str::SplitWhitespace<'_>) -> f32 {
    input.next().expect("missing number").parse().expect("invalid number")
}

fn vector(input: &mut std::str::SplitWhitespace<'_>) -> Vec3 {
    Vec3::new(number(input), number(input), number(input))
}

/// Measures actual production output against caller-supplied reference vectors.
/// Reports exact equality and residuals; finite output alone is not parity acceptance.
#[test]
#[ignore = "requires NFSMW_JUMP_REPLAY numeric boundary fixture"]
fn measure_external_numeric_jump_boundaries() {
    let path = std::env::var_os("NFSMW_JUMP_REPLAY").expect("set NFSMW_JUMP_REPLAY");
    let fixture = std::fs::read_to_string(path).expect("read numeric fixture");
    let mut count = 0;
    let mut exact = 0;
    let mut maximum = 0.0_f32;
    for line in fixture.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
        let mut input = line.split_whitespace();
        let label = input.next().expect("row label");
        let dt = number(&mut input);
        let mass = number(&mut input);
        let inertia = vector(&mut input);
        let rotation = glam::Mat3::from_cols(vector(&mut input), vector(&mut input), vector(&mut input));
        let linear_velocity = vector(&mut input);
        let angular_velocity = vector(&mut input);
        let speed = number(&mut input);
        let grounded_wheels = number(&mut input) as usize;
        let body_contact = number(&mut input) != 0.0;
        let ground = Some(GroundReference { normal: vector(&mut input), separation: number(&mut input) });
        let mut state = JumpState { airborne_time: number(&mut input), peak_altitude: number(&mut input) };
        let extra_airborne_acceleration = number(&mut input);
        let landing = LandingConfig {
            acceleration: number(&mut input),
            minimum_time: number(&mut input),
            minimum_peak_altitude: number(&mut input),
            maximum_altitude: number(&mut input),
            minimum_alignment: number(&mut input),
            maximum_speed: number(&mut input),
        };
        let minimum_up = number(&mut input);
        let maximum_grounded_wheels = number(&mut input) as usize;
        let maximum_speed = number(&mut input);
        let graph_count = number(&mut input) as usize;
        let graph: Vec<_> = (0..graph_count).map(|_| (number(&mut input), number(&mut input))).collect();
        let passive = PassiveConfig {
            minimum_up,
            maximum_grounded_wheels,
            maximum_speed,
            damping: Box::leak(graph.into_boxed_slice()),
        };
        let alignment = AlignmentConfig {
            minimum_ground_up: number(&mut input),
            minimum_body_up: number(&mut input),
            minimum_alignment: number(&mut input),
            minimum_altitude: number(&mut input),
            maximum_altitude: number(&mut input),
            maximum_speed: number(&mut input),
            correction_threshold: number(&mut input),
            pitch: number(&mut input),
            roll: number(&mut input),
        };
        let expected_force = vector(&mut input);
        let expected_torque = vector(&mut input);
        let expected_time = number(&mut input);
        let expected_peak = number(&mut input);
        assert!(input.next().is_none(), "extra input in {label}");
        let result = state.update(
            &JumpConfig { extra_airborne_acceleration, landing, passive, alignment },
            &JumpInput {
                dt,
                mass,
                inertia,
                rotation,
                linear_velocity,
                angular_velocity,
                speed,
                grounded_wheels,
                body_contact,
                ground,
            },
        );
        let residual = (result.force - expected_force)
            .abs()
            .max_element()
            .max((result.torque - expected_torque).abs().max_element())
            .max((state.airborne_time - expected_time).abs())
            .max((state.peak_altitude - expected_peak).abs());
        assert!(residual.is_finite(), "nonfinite result {label}");
        exact += usize::from(
            result.force == expected_force
                && result.torque == expected_torque
                && state.airborne_time == expected_time
                && state.peak_altitude == expected_peak,
        );
        maximum = maximum.max(residual);
        count += 1;
        println!(
            "jump_replay {label} {} {} {} {} {} {} {} {} residual {residual}",
            result.force.x,
            result.force.y,
            result.force.z,
            result.torque.x,
            result.torque.y,
            result.torque.z,
            state.airborne_time,
            state.peak_altitude
        );
    }
    assert!(count > 0, "fixture must contain selected boundaries");
    println!("jump_replay_rows {count} exact {exact} max_absolute {maximum}");
    assert_eq!(exact, count, "numeric jump boundaries must match without a tolerance");
}
