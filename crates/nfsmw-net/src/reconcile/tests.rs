use blackbox_vehicle::InputState;
use nfsmw_sim::toy::Car;

use super::*;
use crate::predict::{Predictor, ToyCar};

/// The thresholds sit above the rounding of the own-car block: an exact prediction compared with its own state sent
/// through the block is always confirmed. The velocity term has the least room: its quantiser has no exact zero and
/// rounds each axis by up to 2.4 cm/s, up to 4.2 cm/s of the 5 cm/s threshold.
#[test]
fn the_rounding_of_the_own_block_alone_never_corrects() {
    use blackbox_net_core::bits::{BitReader, BitWriter};

    let schema = crate::schema::GameSchema::new();
    let mut predictor = Predictor::with_capacity(toy(), 0, 4096);
    let mut rec = Reconciler::default();
    let mut largest = StateError::default();
    for tick in 0..3600 {
        predictor.step(&drive(tick));
        if tick % 3 != 0 {
            continue;
        }
        let state = predictor.entry(tick).unwrap().state;
        let block = OwnBlock {
            position: state.position,
            orientation: state.orientation,
            linear_velocity: state.linear_velocity,
            angular_velocity: state.angular_velocity,
            ..OwnBlock::default()
        };
        let mut bytes = [0u8; 32];
        schema.write_own(&block, &mut BitWriter::new(&mut bytes)).unwrap();
        let back = schema.read_own(&mut BitReader::new(&bytes)).unwrap();
        let auth = AuthState::from_own_block(tick, &back);
        assert_eq!(rec.apply(&mut predictor, &auth, 0), Outcome::Confirmed, "tick {tick}: {:?}", rec.last_error());
        let e = rec.last_error();
        largest.position = largest.position.max(e.position);
        largest.orientation = largest.orientation.max(e.orientation);
        largest.velocity = largest.velocity.max(e.velocity);
    }
    assert!(largest.velocity > 0.03, "the velocity rounding is large: {largest:?}");
}

fn toy() -> ToyCar {
    ToyCar::new(Car::new([0.0, 0.0], [0.0, 1.0], 1.0, 60.0), 500.0)
}

fn drive(tick: u32) -> InputState {
    let t = tick as f32 / 60.0;
    InputState { throttle: 0.8, steer: 0.4 * (t * 0.9).sin(), ..InputState::default() }
}

/// A predictor that has predicted ticks 0..n.
fn predicted(n: u32) -> Predictor<ToyCar> {
    let mut predictor = Predictor::new(toy(), 0);
    for tick in 0..n {
        predictor.step(&drive(tick));
    }
    predictor
}

fn auth(predictor: &Predictor<ToyCar>, tick: u32, dx: f32, cause: CorrectionCause) -> AuthState {
    let mut state = predictor.entry(tick).unwrap().state;
    state.position[0] += dx;
    AuthState { tick, state, cause }
}

#[test]
fn an_exact_prediction_is_confirmed_and_leaves_the_ring() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    let block = auth(&predictor, 20, 0.0, CorrectionCause::None);
    let car_before = predictor.car().car;
    assert_eq!(rec.apply(&mut predictor, &block, 0), Outcome::Confirmed);
    assert_eq!(predictor.car().car, car_before);
    assert_eq!(predictor.oldest(), Some(20));
    assert!(rec.offset().is_zero());
    assert_eq!(rec.stats().checks(), 1);
    assert_eq!(rec.stats().median(), None);
}

#[test]
fn an_error_under_the_thresholds_is_confirmed() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    let block = auth(&predictor, 20, 0.015, CorrectionCause::None);
    assert_eq!(rec.apply(&mut predictor, &block, 0), Outcome::Confirmed);
}

#[test]
fn a_small_error_is_corrected_without_moving_the_drawn_pose() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    let drawn_before = rec.drawn_pose(&predictor);
    let block = auth(&predictor, 20, 0.5, CorrectionCause::None);
    let Outcome::Corrected { distance } = rec.apply(&mut predictor, &block, 0) else {
        panic!("expected a correction");
    };
    assert!((distance - 0.5).abs() < 0.05, "{distance}");
    let drawn_after = rec.drawn_pose(&predictor);
    assert!(geom::distance(drawn_before.position, drawn_after.position) < 1.0e-4);
    assert!(geom::angle_between(drawn_before.orientation, drawn_after.orientation) < 1.0e-3);
    // One half-life later, half of it is left.
    rec.advance(50.0);
    assert!((rec.offset().distance() - distance * 0.5).abs() < 0.01, "{}", rec.offset().distance());
    // A second later it is gone.
    for _ in 0..60 {
        rec.advance(1000.0 / 60.0);
    }
    assert!(rec.offset().distance() < 1.0e-4);
    assert_eq!(rec.stats().corrections(), 1);
}

#[test]
fn the_offset_shrinks_every_frame_and_never_jumps() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    {
        let block = auth(&predictor, 20, 1.5, CorrectionCause::None);
        rec.apply(&mut predictor, &block, 0)
    };
    let mut last = rec.offset().distance();
    for _ in 0..30 {
        rec.advance(1000.0 / 60.0);
        let now = rec.offset().distance();
        assert!(now < last);
        // At 60 fps a 50 ms half-life removes about 21 % per frame: no visible step.
        assert!(last - now < last * 0.25);
        last = now;
    }
}

#[test]
fn a_correction_past_the_snap_distance_snaps() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    let block = auth(&predictor, 20, 3.0, CorrectionCause::None);
    let outcome = rec.apply(&mut predictor, &block, 0);
    assert!(matches!(outcome, Outcome::Snapped { reason: SnapReason::Distance, .. }), "{outcome:?}");
    assert!(rec.offset().is_zero());
    assert_eq!(rec.stats().snaps(), 1);
}

#[test]
fn reset_and_teleport_always_snap() {
    for (cause, reason) in
        [(CorrectionCause::Reset, SnapReason::Reset), (CorrectionCause::Teleport, SnapReason::Teleport)]
    {
        let mut predictor = predicted(30);
        let mut rec = Reconciler::default();
        let block = auth(&predictor, 20, 0.0, cause);
        let outcome = rec.apply(&mut predictor, &block, 0);
        assert!(matches!(outcome, Outcome::Snapped { reason: r, .. } if r == reason), "{outcome:?}");
    }
}

#[test]
fn a_contact_reconciles_normally() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    let block = auth(&predictor, 20, 0.3, CorrectionCause::Contact);
    assert!(matches!(rec.apply(&mut predictor, &block, 0), Outcome::Corrected { .. }));
}

#[test]
fn freeze_holds_inputs_neutral_until_released() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    {
        let block = auth(&predictor, 20, 0.0, CorrectionCause::Freeze);
        rec.apply(&mut predictor, &block, 0)
    };
    assert!(predictor.is_frozen());
    {
        let block = auth(&predictor, 23, 0.0, CorrectionCause::None);
        rec.apply(&mut predictor, &block, 0)
    };
    assert!(!predictor.is_frozen());
}

#[test]
fn an_older_or_repeated_block_is_stale() {
    let mut predictor = predicted(30);
    let mut rec = Reconciler::default();
    {
        let block = auth(&predictor, 20, 0.0, CorrectionCause::None);
        rec.apply(&mut predictor, &block, 0)
    };
    let old = AuthState { tick: 18, state: Default::default(), cause: CorrectionCause::None };
    assert_eq!(rec.apply(&mut predictor, &old, 0), Outcome::Stale);
    let repeated = AuthState { tick: 20, state: Default::default(), cause: CorrectionCause::None };
    assert_eq!(rec.apply(&mut predictor, &repeated, 0), Outcome::Stale);
}

#[test]
fn a_tick_before_the_prediction_started_is_stale() {
    let mut predictor = Predictor::new(toy(), 100);
    predictor.step(&drive(100));
    let mut rec = Reconciler::default();
    let early = AuthState { tick: 90, state: Default::default(), cause: CorrectionCause::None };
    assert_eq!(rec.apply(&mut predictor, &early, 0), Outcome::Stale);
}

#[test]
fn a_tick_past_the_prediction_puts_the_car_there() {
    let mut predictor = predicted(10);
    let mut rec = Reconciler::default();
    let state = crate::predict::CarState { position: [5.0, 0.0, 5.0], ..Default::default() };
    let outcome = rec.apply(&mut predictor, &AuthState { tick: 40, state, cause: CorrectionCause::None }, 0);
    assert!(matches!(outcome, Outcome::Snapped { reason: SnapReason::Unpredicted, .. }), "{outcome:?}");
    assert_eq!(predictor.next_tick(), 41);
    assert_eq!(predictor.car().car.pos, [5.0, 5.0]);
}

#[test]
fn a_clock_step_clears_the_prediction() {
    let mut predictor = predicted(10);
    let mut rec = Reconciler::default();
    rec.clock_step(&mut predictor, 500, 0);
    assert_eq!(predictor.unacknowledged(), 0);
    assert_eq!(predictor.next_tick(), 500);
    assert_eq!(rec.stats().snaps(), 1);
}

#[test]
fn percentiles_and_rates() {
    let mut stats = CorrectionStats::default();
    for i in 0..100 {
        stats.record_correction(i as f32 / 100.0, i * 1000);
    }
    assert_eq!(stats.median(), Some(0.5));
    assert_eq!(stats.p95(), Some(0.94));
    assert_eq!(stats.corrections_per_minute(99_000), 60);
    stats.record_snap(3.0, 99_000);
    assert_eq!(stats.snaps_per_minute(99_000), 1);
    assert_eq!(stats.snaps_per_minute(200_000), 0);
    assert_eq!(stats.largest(), 3.0);
}

/// The same server states applied to the same prediction give bit-identical results.
#[test]
fn reconciliation_is_deterministic() {
    let run = || {
        let mut predictor = Predictor::new(toy(), 0);
        let mut rec = Reconciler::default();
        let mut outcomes = Vec::new();
        for tick in 0..600u32 {
            predictor.step(&drive(tick));
            rec.advance(1000.0 / 60.0);
            if tick % 3 == 0 && tick >= 10 {
                let at = tick - 6;
                let wobble = ((at * 7919) % 13) as f32 * 0.01;
                outcomes.push({
                    let block = auth(&predictor, at, wobble, CorrectionCause::None);
                    rec.apply(&mut predictor, &block, 0)
                });
            }
        }
        (outcomes, predictor.car().car, rec.drawn_pose(&predictor))
    };
    let (a, car_a, drawn_a) = run();
    let (b, car_b, drawn_b) = run();
    assert_eq!(a, b);
    assert_eq!(car_a, car_b);
    assert_eq!(drawn_a, drawn_b);
}
