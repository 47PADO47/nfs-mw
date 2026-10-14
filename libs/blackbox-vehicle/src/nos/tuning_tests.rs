//! All eleven power/duration positions from isolated original-helper replay.

use super::{Nos, NosInput, NosSpec};
use crate::drivetrain::GEAR_FIRST;

#[test]
fn eleven_tuning_positions_match_the_power_and_duration_tradeoff() {
    let boosts = [1.375, 1.400, 1.425, 1.450, 1.475, 1.500, 1.525, 1.550, 1.575, 1.600, 1.625];
    let durations = [6.25, 6.00, 5.75, 5.50, 5.25, 5.00, 4.75, 4.50, 4.25, 4.00, 3.75];
    let spec = NosSpec { nos_capacity: 5.0, torque_boost: 0.5, ..Default::default() };
    for (index, (boost, duration)) in boosts.into_iter().zip(durations).enumerate() {
        let mut state = Nos::new(&spec);
        state.capacity = 0.5;
        state.update(
            &spec,
            &NosInput {
                dt: 0.1,
                held: true,
                gear: GEAR_FIRST,
                throttle: 1.0,
                speed_mph: 60.0,
                blown: false,
                tuning: (index as f32 - 5.0) / 5.0,
                can_recharge: true,
            },
        );
        assert!((state.boost - boost).abs() < 1e-6, "position={index}");
        assert!((state.capacity - (0.5 - 0.1 / duration)).abs() < 1e-6, "position={index}");
    }
}
