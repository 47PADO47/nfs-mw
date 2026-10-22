//! Quantisation error bounds: every decoded value is within half a step of the original (one step for positions,
//! whose range is not a multiple of the step), over seeded random values.

use blackbox_net_core::bits::Quant;
use blackbox_net_core::rng::SplitMix64;

use super::super::quant::{
    self, ANGULAR_VELOCITY, LINEAR_VELOCITY, POLICE_SPEED, REDUCED_SPEED, RPM, TRAFFIC_SPEED, pack_yaw, unpack_yaw,
};
use super::super::*;

fn uniform(rng: &mut SplitMix64, min: f32, max: f32) -> f32 {
    min + (max - min) * rng.next_f64() as f32
}

fn check_bound(quant: &Quant, min: f32, max: f32) {
    let mut rng = SplitMix64(u64::from(quant.bits()) * 7919);
    let bound = quant.step() * 0.5 + max.abs().max(min.abs()) * 1e-6;
    for _ in 0..5_000 {
        let x = uniform(&mut rng, min, max);
        let back = quant.decode(quant.encode(x)).unwrap();
        assert!((back - x).abs() <= bound, "{x} came back as {back}, bound {bound}");
    }
}

#[test]
fn uniform_fields_stay_within_half_a_step() {
    check_bound(&LINEAR_VELOCITY, -400.0, 400.0);
    check_bound(&ANGULAR_VELOCITY, -16.0, 16.0);
    check_bound(&RPM, 0.0, 10_230.0);
    check_bound(&TRAFFIC_SPEED, -32.0, 96.0);
    check_bound(&POLICE_SPEED, -32.0, 96.0);
    check_bound(&REDUCED_SPEED, -32.0, 96.0);
    assert!(LINEAR_VELOCITY.step() <= 0.05);
    assert!(ANGULAR_VELOCITY.step() <= 0.032);
    assert!((RPM.step() - 10.0).abs() < 1e-3);
}

#[test]
fn positions_stay_within_a_centimetre() {
    for half_extent in [GameSchema::DEFAULT_HALF_EXTENT, 1000.0] {
        let schema = GameSchema::with_half_extent(half_extent).unwrap();
        assert!(schema.position_step() <= 2.0 * half_extent / ((1 << POSITION_BITS) - 1) as f32 + 1e-6);
        let mut rng = SplitMix64(42);
        let mut record = Record::default();
        for _ in 0..5_000 {
            let mut car = super::view(nfsmw_sim::EntityKind::PlayerCar);
            car.position = [0, 1, 2].map(|_| uniform(&mut rng, -half_extent, half_extent));
            schema.encode(&car, None, &mut record).unwrap();
            let back = schema.decode(car.id, record.kind(), record.values()).unwrap().view.position;
            for axis in 0..3 {
                let error = (back[axis] - car.position[axis]).abs();
                assert!(error <= schema.position_step() * 0.5 + half_extent * 1e-6, "error {error}");
            }
        }
        assert!(GameSchema::new().position_step() < 0.02);
    }
}

#[test]
fn orientations_stay_within_half_a_degree() {
    let mut rng = SplitMix64(7);
    for _ in 0..5_000 {
        let raw = [0, 1, 2, 3].map(|_| uniform(&mut rng, -1.0, 1.0));
        let norm = raw.iter().map(|c| c * c).sum::<f32>().sqrt();
        if norm < 0.1 {
            continue;
        }
        let q = raw.map(|c| c / norm);
        let back = unpack_quat(pack_quat(q).unwrap()).unwrap();
        let dot: f32 = q.iter().zip(back).map(|(a, b)| a * b).sum();
        let angle = 2.0 * dot.abs().min(1.0).acos();
        assert!(angle < 0.5_f32.to_radians(), "angle {angle}");
    }
}

#[test]
fn yaws_stay_within_half_a_step() {
    let mut rng = SplitMix64(11);
    for bits in [8u32, 10] {
        let step = std::f32::consts::TAU / (1u32 << bits) as f32;
        for _ in 0..5_000 {
            let yaw = uniform(&mut rng, -std::f32::consts::PI, std::f32::consts::PI);
            let back = unpack_yaw(pack_yaw(quat_of_yaw(yaw), bits).unwrap(), bits);
            let error = (back - yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            assert!(error.abs() <= step * 0.5 + 1e-5, "yaw {yaw} back {back}");
        }
    }
    assert!((yaw_of(quat_of_yaw(1.0)) - 1.0).abs() < 1e-6);
    assert!((quant::forward_speed(quant::velocity_of(12.0, 2.0), 2.0) - 12.0).abs() < 1e-4);
}

#[test]
fn packing_round_trips() {
    let codes = [63, 0, 17, 42];
    assert_eq!(quant::unpack::<4>(quant::pack(&codes, 6), 6), codes);
}
