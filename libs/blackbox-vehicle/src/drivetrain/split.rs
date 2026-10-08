use super::diff::calc_split;
use super::spec::TransmissionSpec;

/// Splits `drive_torque` (N m at the wheels) over the four wheels: first between the axles (fixed split or
/// a centre differential), then across each axle through its differential. Only grounded wheels receive
/// torque. Wheel order is front left, front right, rear left, rear right.
pub fn split_drive_torque(
    drive_torque: f32,
    trans: &TransmissionSpec,
    omega: [f32; 4],
    grounded: [bool; 4],
) -> [f32; 4] {
    let mut out = [0.0; 4];
    if drive_torque.abs() <= f32::EPSILON {
        return out;
    }
    let share = if trans.differential[2] > 0.0 {
        calc_split(
            trans.torque_split,
            trans.differential[2],
            false,
            [grounded[0] || grounded[1], grounded[2] || grounded[3]],
            [omega[0] + omega[1], omega[2] + omega[3]],
        )
    } else {
        [trans.torque_split, 1.0 - trans.torque_split]
    };
    for (axle, &axle_share) in share.iter().enumerate() {
        let axle_torque = drive_torque * axle_share;
        if axle_torque.abs() <= f32::EPSILON {
            continue;
        }
        let (a, b) = (axle * 2, axle * 2 + 1);
        let s = calc_split(0.5, trans.differential[axle], false, [grounded[a], grounded[b]], [omega[a], omega[b]]);
        if grounded[a] {
            out[a] = axle_torque * s[0];
        }
        if grounded[b] {
            out[b] = axle_torque * s[1];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trans(split: f32, diff: [f32; 3]) -> TransmissionSpec {
        TransmissionSpec {
            gear_ratio: vec![3.0, 0.0, 3.0],
            gear_efficiency: vec![1.0; 3],
            final_gear: 3.5,
            torque_split: split,
            differential: diff,
            torque_converter: 0.0,
            clutch_slip: 0.0,
            shift_speed: 0.1,
            optimal_shift: 0.0,
        }
    }

    #[test]
    fn rear_drive_goes_to_the_rear_pair() {
        let t = split_drive_torque(1000.0, &trans(0.0, [0.0; 3]), [0.0; 4], [true; 4]);
        assert_eq!(t, [0.0, 0.0, 500.0, 500.0]);
    }

    #[test]
    fn four_wheel_drive_splits_by_the_bias() {
        let t = split_drive_torque(1000.0, &trans(0.4, [0.0; 3]), [0.0; 4], [true; 4]);
        assert_eq!(t, [200.0, 200.0, 300.0, 300.0]);
    }

    #[test]
    fn airborne_wheels_get_nothing() {
        let t = split_drive_torque(1000.0, &trans(0.0, [0.0; 3]), [0.0; 4], [true, true, true, false]);
        assert_eq!(t, [0.0, 0.0, 500.0, 0.0]);
    }

    #[test]
    fn limited_slip_feeds_the_slow_wheel() {
        let t = split_drive_torque(1000.0, &trans(0.0, [0.0, 1.0, 0.0]), [0.0, 0.0, 40.0, 20.0], [true; 4]);
        assert!(t[3] > t[2]);
        assert!((t[2] + t[3] - 1000.0).abs() < 1e-3);
    }
}
