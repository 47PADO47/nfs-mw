use super::*;

#[test]
fn reset_collision_timer_counts_only_after_a_strict_contact_free_second() {
    let mut state = ResetCollision { remaining: 2.0, ..Default::default() };
    state.advance(1.0);
    assert_eq!(state.remaining, 2.0, "exactly one second does not pass the native strict gate");
    state.advance(0.25);
    assert_eq!(state.remaining, 1.75);
    assert!(!state.admit());
    state.advance(1.0);
    assert_eq!(state.remaining, 1.75, "refused overlap pauses countdown for another full second");
    state.advance(0.25);
    assert_eq!(state.remaining, 1.5);
    state.advance(1.5);
    assert!(!state.active());
    assert!(state.admit());
}
