//! Synthetic numeric cases replayed through the original boost helper.

use super::{Induction, InductionInput, InductionSpec};

#[test]
fn eleven_tuning_positions_match_turbo_and_supercharger_boost_vectors() {
    let supercharger = [0.275, 0.280, 0.285, 0.290, 0.295, 0.300, 0.305, 0.310, 0.315, 0.320, 0.325];
    let turbo = [
        0.260,
        0.253_333_33,
        0.244_347_83,
        0.232_727_27,
        0.218_095_24,
        0.200,
        -0.076_190_47,
        -0.072_727_27,
        -0.069_565_22,
        -0.066_666_66,
        -0.064,
    ];
    for (spool, expected) in [(0.0, supercharger), (0.5, turbo)] {
        let spec = InductionSpec {
            low_boost: 0.2,
            high_boost: 0.4,
            spool,
            spool_time_up: 0.2,
            spool_time_down: 0.2,
            vacuum: -0.08,
            psi: 16.0,
        };
        for (index, expected) in expected.into_iter().enumerate() {
            // Hold full spool with a zero-duration sample to isolate the helper's RPM/tuning curve.
            let mut state = Induction { spool: 1.0, ..Default::default() };
            state.update(
                &spec,
                &InductionInput {
                    dt: 0.0,
                    throttle: 1.0,
                    rpm: 4000.0,
                    idle: 1000.0,
                    red_line: 7000.0,
                    shifting: false,
                    tuning: (index as f32 - 5.0) / 5.0,
                },
            );
            assert!((state.boost - expected).abs() < 1e-6, "spool={spool}, position={index}: {}", state.boost);
        }
    }
}
