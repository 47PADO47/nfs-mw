use super::super::FixedClock;
use super::*;

// Float bit patterns of six successive unchanged original-PC time-manager outputs.
// Each tuple is (stored multiplier, scheduler period times final multiplier).
const ENTRY_60: [(u32, u32); 6] = [
    (0x3f777777, 0x3c83fb73),
    (0x3f6eeeee, 0x3c7edcba),
    (0x3f666665, 0x3c75c28f),
    (0x3f5ddddc, 0x3c6ca863),
    (0x3f555553, 0x3c638e37),
    (0x3f4cccca, 0x3c5a740b),
];

const EXIT_60: [(u32, u32); 6] = [
    (0x3e844444, 0x3b8d159e),
    (0x3e888888, 0x3b91a2b4),
    (0x3e8ccccc, 0x3b962fc9),
    (0x3e911110, 0x3b9abcde),
    (0x3e955554, 0x3b9f49f4),
    (0x3e999998, 0x3ba3d709),
];

const ENTRY_144: [(u32, u32); 6] = [
    (0x3f7c71c7, 0x3be06523),
    (0x3f78e38e, 0x3bdd3c0d),
    (0x3f755555, 0x3bda12f6),
    (0x3f71c71c, 0x3bd6e9e0),
    (0x3f6e38e3, 0x3bd3c0ca),
    (0x3f6aaaaa, 0x3bd097b4),
];

const EXIT_144: [(u32, u32); 6] = [
    (0x3e81c71c, 0x3ae6b74e),
    (0x3e838e38, 0x3ae9e064),
    (0x3e855554, 0x3aed0979),
    (0x3e871c70, 0x3af0328e),
    (0x3e88e38c, 0x3af35ba4),
    (0x3e8aaaa8, 0x3af684b9),
];

#[test]
fn completed_ticks_match_original_entry_and_exit_float_vectors() {
    for (rate, active, vectors) in
        [(60, true, ENTRY_60), (60, false, EXIT_60), (144, true, ENTRY_144), (144, false, EXIT_144)]
    {
        let mut ramp = SpeedbreakerClock::default();
        if !active {
            ramp.physics_multiplier = 0.25;
        }
        for (multiplier, delta) in vectors {
            let actual = ramp.advance_tick(active, 1.0 / rate as f32);
            assert_eq!(actual.to_bits(), delta, "rate {rate}, active {active}");
            assert_eq!(ramp.physics_multiplier.to_bits(), multiplier);
        }
    }
}

#[test]
fn endpoint_sequences_are_identical_across_render_frame_groupings() {
    for rate in [60, 144] {
        for active in [true, false] {
            let run = |fps| {
                let mut clock = FixedClock::default();
                clock.set_rate_hz(rate);
                let mut ramp = SpeedbreakerClock::default();
                if !active {
                    ramp.physics_multiplier = 0.25;
                }
                (0..fps * 2)
                    .flat_map(|_| clock.advance_real(1.0 / fps as f32))
                    .map(|period| ramp.advance_tick(active, period).to_bits())
                    .collect::<Vec<_>>()
            };
            let reference = run(60);
            assert_eq!(reference.len(), rate as usize * 2);
            for fps in [5, 30, 120, 144, 240] {
                assert_eq!(run(fps), reference, "rate {rate}, active {active}, FPS {fps}");
            }
        }
    }
}

#[test]
fn quarter_speed_preserves_the_real_cadence_at_both_selected_rates() {
    for rate in [60, 144] {
        let mut clock = FixedClock::default();
        clock.set_rate_hz(rate);
        let mut ramp = SpeedbreakerClock { physics_multiplier: 0.25, ..Default::default() };
        let deltas: Vec<_> = (0..240)
            .flat_map(|_| clock.advance_real(1.0 / 240.0))
            .map(|period| ramp.advance_tick(true, period))
            .collect();
        assert_eq!(deltas.len(), rate as usize);
        assert!(deltas.iter().all(|&delta| delta == (1.0 / rate as f32) * 0.25));
    }
}

#[test]
fn continuous_frame_integral_is_separate_from_endpoint_physics_deltas() {
    let mut clock = FixedClock::default();
    let mut ramp = SpeedbreakerClock::default();
    let real = f64::from(0.1_f32);
    let frame_sim = ramp.advance_f64(true, real);
    let deltas: Vec<_> =
        clock.advance_real(real as f32).into_iter().map(|period| ramp.advance_tick(true, period)).collect();
    assert_eq!(deltas.len(), 6);
    assert!(deltas.windows(2).all(|pair| pair[0] > pair[1]));
    assert!((frame_sim - (real - real * real)).abs() < 1e-14);
    assert!(f64::from(deltas.iter().sum::<f32>()) < frame_sim);
    assert_eq!(deltas[0].to_bits(), ENTRY_60[0].1);
}

#[test]
fn partial_render_frame_does_not_partly_update_the_physics_multiplier() {
    let mut clock = FixedClock::default();
    let mut ramp = SpeedbreakerClock::default();
    let half = super::super::clock::STEP * 0.5;
    ramp.advance_f64(true, f64::from(half));
    assert!(clock.advance_real(half).is_empty());
    assert_eq!(ramp.physics_multiplier, 1.0);
    // The action is released before the completed tick. The tick samples its final active state.
    let periods = clock.advance_real(half);
    assert_eq!(periods.len(), 1);
    assert_eq!(ramp.advance_tick(false, periods[0]), super::super::clock::STEP);
}

#[test]
fn invalid_ticks_and_network_reset_preserve_the_unscaled_contract() {
    let mut ramp = SpeedbreakerClock { physics_multiplier: 0.25, ..Default::default() };
    ramp.advance_f64(true, 1.0);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(ramp.advance_tick(false, dt), 0.0);
    }
    assert_eq!(ramp.physics_multiplier, 0.25);
    ramp.reset();
    assert_eq!(ramp.advance_f64(false, 1.0 / 60.0), 1.0 / 60.0);
    assert_eq!(ramp.advance_tick(false, super::super::clock::STEP), super::super::clock::STEP);
}
