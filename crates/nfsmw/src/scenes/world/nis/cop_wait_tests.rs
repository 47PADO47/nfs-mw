use super::*;

#[test]
fn a_wait_holds_its_slots_in_order_until_they_resolve() {
    let mut wait = CopWait::new(vec![8, 10, 12]);
    assert_eq!(wait.slots(), &[8, 10, 12]);
    wait.resolve(10);
    assert_eq!(wait.slots(), &[8, 12], "the channel that got its car is off the wait");
    wait.resolve(10);
    assert_eq!(wait.slots(), &[8, 12], "resolving a slot twice is harmless");
}

#[test]
fn an_empty_wait_never_gives_up() {
    let mut wait = CopWait::new(Vec::new());
    assert!(wait.tick(10.0).is_empty(), "no channel waits, so none gives up");
    assert_eq!(wait, CopWait::default());
}

#[test]
fn the_slots_wait_while_the_time_is_under_the_limit() {
    let mut wait = CopWait::new(vec![9]);
    assert!(wait.tick(0.5).is_empty());
    assert!(wait.tick(WAIT_LIMIT - 1.0).is_empty(), "under the limit: still waiting");
    assert_eq!(wait.slots(), &[9]);
}

#[test]
fn the_slots_give_up_once_the_limit_is_reached_and_only_once() {
    let mut wait = CopWait::new(vec![9, 11]);
    assert!(wait.tick(1.0).is_empty());
    assert_eq!(wait.tick(1.0), vec![9, 11], "two seconds of real time give up both channels");
    assert!(wait.slots().is_empty());
    assert!(wait.tick(1.0).is_empty(), "the give-up is reported once");
}

#[test]
fn a_long_frame_gives_up_at_once() {
    let mut wait = CopWait::new(vec![15]);
    assert_eq!(wait.tick(WAIT_LIMIT + 0.5), vec![15]);
}

#[test]
fn a_channel_that_gets_its_car_in_time_is_not_given_up() {
    let mut wait = CopWait::new(vec![8, 9]);
    assert!(wait.tick(1.5).is_empty());
    wait.resolve(8);
    wait.resolve(9);
    assert!(wait.tick(1.0).is_empty(), "both got cars, so nothing gives up even past the limit");
}
