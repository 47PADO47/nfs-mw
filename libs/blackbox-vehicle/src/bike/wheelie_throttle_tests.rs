use super::*;

fn ask() -> Ask {
    Ask { weight: 0.0, throttle: 1.0, front_brake: 0.0, speed: 10.0, pitch: 0.0, lean_share: 0.0, held: false }
}

#[test]
fn flat_out_with_the_lever_at_rest_pulls_the_virtual_lever_back() {
    let game = ThrottleWheelie::GAME;
    assert_eq!(lever(&game, &ask()), (-1.0, true));
    assert_eq!(lever(&game, &Ask { throttle: 0.95, ..ask() }), (-1.0, true));
}

#[test]
fn a_throttle_rolled_on_or_shut_asks_for_nothing() {
    let game = ThrottleWheelie::GAME;
    assert_eq!(lever(&game, &Ask { throttle: 0.5, ..ask() }), (0.0, false));
    assert_eq!(lever(&game, &Ask { throttle: 0.0, ..ask() }), (0.0, false));
}

#[test]
fn the_riders_own_lever_the_front_brake_a_lean_and_a_fast_road_are_left_alone() {
    let game = ThrottleWheelie::GAME;
    assert_eq!(lever(&game, &Ask { weight: 0.6, ..ask() }), (0.6, false), "a stoppie lever is the rider's");
    assert_eq!(lever(&game, &Ask { weight: -0.5, ..ask() }), (-0.5, false), "so is a real wheelie lever");
    assert_eq!(lever(&game, &Ask { front_brake: 0.5, ..ask() }), (0.0, false));
    assert_eq!(lever(&game, &Ask { lean_share: 0.8, ..ask() }), (0.0, false), "no wheelie in a corner");
    assert_eq!(lever(&game, &Ask { speed: 55.0, ..ask() }), (0.0, false), "none is started above the top speed");
}

#[test]
fn a_wheelie_that_is_up_stays_asked_for_until_the_nose_is_down() {
    let game = ThrottleWheelie::GAME;
    let up = Ask { throttle: 0.0, pitch: 0.4, held: true, ..ask() };
    assert_eq!(lever(&game, &up), (-1.0, true), "the throttle shut: the nose is let down gently");
    assert_eq!(lever(&game, &Ask { pitch: 0.02, ..up }), (0.0, false), "down: the lever is released");
    assert_eq!(lever(&game, &Ask { held: false, ..up }), (0.0, false), "a wheelie nobody asked for is not held");
    assert_eq!(lever(&game, &Ask { speed: 55.0, ..up }), (-1.0, true), "a wheelie up is held at any speed");
    assert_eq!(lever(&game, &Ask { front_brake: 0.5, ..up }), (0.0, false), "the front brake ends it");
}

#[test]
fn the_reference_values_give_the_reference_behaviour() {
    let off = ThrottleWheelie::REFERENCE;
    assert!(!off.enabled);
    for weight in [-1.0, -0.3, 0.0, 0.4] {
        let held = Ask { weight, pitch: 0.5, held: true, ..ask() };
        assert_eq!(lever(&off, &held), (weight, false));
        assert_eq!(lever(&off, &Ask { weight, ..ask() }), (weight, false));
    }
}
