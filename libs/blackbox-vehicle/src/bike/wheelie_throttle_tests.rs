use super::*;

const DT: f32 = 1.0 / 60.0;

fn ask() -> Ask {
    Ask {
        weight: 0.0,
        throttle: 1.0,
        front_brake: 0.0,
        speed: 10.0,
        pitch: 0.0,
        lean_share: 0.0,
        gear: 1,
        shifting: false,
        front_on_ground: true,
        dt: DT,
    }
}

fn degrees(rad: f32) -> f32 {
    rad.to_degrees()
}

/// Run `steps` steps of `s` with the nose following the allowed angle as an ideal bike does (the front wheel is on the road under 3.4
/// degrees); the highest allowed angle seen.
fn ride(game: &ThrottleWheelie, state: &mut State, s: Ask, steps: usize) -> f32 {
    let mut high = 0.0_f32;
    let mut pitch = s.pitch;
    for _ in 0..steps {
        let lever = update(game, state, &Ask { pitch, front_on_ground: pitch < 0.06, ..s });
        if lever.active {
            pitch = lever.reach;
            high = high.max(lever.reach);
        }
    }
    high
}

#[test]
fn flat_out_with_the_lever_at_rest_pulls_the_virtual_lever_back() {
    let game = ThrottleWheelie::GAME;
    let lever = update(&game, &mut State::default(), &ask());
    assert_eq!((lever.weight, lever.active), (-1.0, true));
    let lever = update(&game, &mut State::default(), &Ask { throttle: 0.95, ..ask() });
    assert_eq!((lever.weight, lever.active), (-1.0, true));
}

#[test]
fn a_throttle_rolled_on_or_shut_asks_for_nothing() {
    let game = ThrottleWheelie::GAME;
    for throttle in [0.5, 0.0] {
        let lever = update(&game, &mut State::default(), &Ask { throttle, ..ask() });
        assert_eq!((lever.weight, lever.active), (0.0, false));
    }
}

#[test]
fn the_riders_own_lever_the_front_brake_a_lean_and_a_fast_road_are_left_alone() {
    let game = ThrottleWheelie::GAME;
    let rider = |s: Ask| {
        let lever = update(&game, &mut State::default(), &s);
        (lever.weight, lever.active, lever.reach.is_infinite())
    };
    assert_eq!(rider(Ask { weight: 0.6, ..ask() }), (0.6, false, true), "a stoppie lever is the rider's");
    assert_eq!(rider(Ask { weight: -0.5, ..ask() }), (-0.5, false, true), "so is a real wheelie lever");
    assert_eq!(rider(Ask { front_brake: 0.5, ..ask() }), (0.0, false, true));
    assert_eq!(rider(Ask { lean_share: 0.8, ..ask() }), (0.0, false, true), "no wheelie in a corner");
    assert_eq!(rider(Ask { speed: 55.0, ..ask() }), (0.0, false, true), "none is started above the top speed");
}

#[test]
fn a_wheelie_that_is_up_stays_asked_for_until_the_nose_is_down() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    let up = Ask { pitch: 0.4, front_on_ground: false, ..ask() };
    assert!(update(&game, &mut state, &up).active);
    let shut = Ask { throttle: 0.0, ..up };
    assert!(update(&game, &mut state, &shut).active, "the throttle shut: the nose is let down gently");
    assert!(update(&game, &mut state, &Ask { speed: 55.0, ..shut }).active, "a wheelie up is held at any speed");
    let lever = update(&game, &mut state, &Ask { pitch: 0.02, ..shut });
    assert!(!lever.active, "down: the lever is released");
    let mut other = State::default();
    assert!(!update(&game, &mut other, &shut).active, "a wheelie nobody asked for is not held");
}

#[test]
fn the_front_brake_and_the_riders_lever_end_a_wheelie_and_latch_it() {
    let game = ThrottleWheelie::GAME;
    for ended in [Ask { front_brake: 0.5, ..ask() }, Ask { weight: -0.8, ..ask() }, Ask { weight: 0.8, ..ask() }] {
        let mut state = State::default();
        let up = Ask { pitch: 0.4, front_on_ground: false, ..ask() };
        assert!(update(&game, &mut state, &up).active && update(&game, &mut state, &up).active);
        assert!(!update(&game, &mut state, &Ask { pitch: 0.4, front_on_ground: false, ..ended }).active);
        assert!(!update(&game, &mut state, &up).active, "the gas held again does not lift it again: {ended:?}");
    }
}

#[test]
fn the_cap_of_each_gear_is_never_exceeded_and_a_higher_gear_asks_for_nothing() {
    let game = ThrottleWheelie::GAME;
    for (gear, cap) in [(1, 40.0), (2, 35.0), (3, 30.0)] {
        let mut state = State::default();
        let high = ride(&game, &mut state, Ask { gear, ..ask() }, 60 * 12);
        assert!((degrees(high) - cap).abs() < 0.1, "gear {gear}: the allowed angle reaches {} degrees", degrees(high));
    }
    for gear in [4, 5, 6] {
        let lever = update(&game, &mut State::default(), &Ask { gear, ..ask() });
        assert!(!lever.active, "gear {gear} asks for no wheelie");
    }
}

#[test]
fn a_wheelie_up_when_the_gear_goes_beyond_the_third_comes_down_and_is_put_down() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, Ask { gear: 3, ..ask() }, 60 * 8);
    let mut pitch = state.reach;
    let mut came_down = false;
    for _ in 0..(3.0 / DT) as usize {
        let lever = update(&game, &mut state, &Ask { gear: 4, pitch, front_on_ground: pitch < 0.06, ..ask() });
        if !lever.active {
            came_down = true;
            break;
        }
        assert!(lever.reach <= pitch + 1e-6, "the allowed angle only falls: {} after {}", lever.reach, pitch);
        pitch = lever.reach;
    }
    assert!(came_down && !state.asked && state.latched, "up in fourth gear: lowered and put down");
}

#[test]
fn the_cap_does_not_apply_to_the_riders_own_lever() {
    let game = ThrottleWheelie::GAME;
    let manual = Ask { gear: 3, weight: -1.0, pitch: 0.7, front_on_ground: false, ..ask() };
    let lever = update(&game, &mut State::default(), &manual);
    assert!(!lever.active && lever.reach.is_infinite() && lever.cut == 0.0);
}

#[test]
fn the_wheelie_rises_slowly() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, ask(), 60);
    let after_one_second = degrees(state.reach);
    assert!(after_one_second < 8.5 && after_one_second > 7.0, "{after_one_second} degrees after 1 s");
    // 10 degrees take over a second; the cap of first gear 5 s.
    let mut state = State::default();
    ride(&game, &mut state, ask(), 70);
    assert!(degrees(state.reach) < 10.0 + 0.5);
    let mut state = State::default();
    ride(&game, &mut state, ask(), 60 * 4);
    assert!(degrees(state.reach) < 39.9, "not at the cap after 4 s: {}", degrees(state.reach));
}

#[test]
fn the_allowed_angle_does_not_run_away_from_a_nose_that_stays_low() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    let mut last = 0.0;
    for _ in 0..60 * 6 {
        last = update(&game, &mut state, &Ask { pitch: 0.05, front_on_ground: true, ..ask() }).reach;
    }
    assert!(last <= 0.05 + game.lead + 1e-6, "{} degrees over a nose of 3", degrees(last));
}

#[test]
fn a_gear_change_dips_the_allowed_angle_then_lets_it_rise_again() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, Ask { gear: 2, ..ask() }, 60 * 8);
    let level = state.reach;
    assert!((degrees(level) - 35.0).abs() < 0.1);
    // The change begins: the allowed angle falls by the dip while it lasts (the nose follows it down).
    let mut pitch = level;
    let mut lowest = level;
    for step in 0..(game.shift_dip_time / DT).ceil() as usize + 6 {
        let shifting = step < 6;
        let lever = update(&game, &mut state, &Ask { gear: 2, shifting, pitch, front_on_ground: false, ..ask() });
        pitch = lever.reach;
        lowest = lowest.min(lever.reach);
    }
    assert!(lowest < level - game.shift_dip * 0.9, "dipped {} degrees", degrees(level - lowest));
    assert!(lowest > level - game.shift_dip - 1e-3, "a little: not under {} degrees", degrees(game.shift_dip));
    // After it the angle rises again, at the slow rate, back to the cap.
    let low = state.reach;
    for _ in 0..60 {
        pitch = update(&game, &mut state, &Ask { gear: 2, pitch, front_on_ground: false, ..ask() }).reach;
    }
    assert!(
        state.reach > low + 0.05 && state.reach <= level + 1e-6,
        "{} to {} degrees",
        degrees(low),
        degrees(state.reach)
    );
    for _ in 0..60 * 4 {
        pitch = update(&game, &mut state, &Ask { gear: 2, pitch, front_on_ground: false, ..ask() }).reach;
    }
    assert!((degrees(state.reach) - 35.0).abs() < 0.1);
}

#[test]
fn a_gear_change_before_the_cap_is_reached_dips_from_where_the_wheelie_is() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, ask(), 60 * 2);
    let level = state.reach;
    assert!(degrees(level) > 14.0 && degrees(level) < 18.0, "{} degrees after 2 s", degrees(level));
    let mut pitch = level;
    let mut lowest = level;
    for _ in 0..20 {
        let lever = update(&game, &mut state, &Ask { shifting: true, pitch, front_on_ground: false, ..ask() });
        pitch = lever.reach;
        lowest = lowest.min(lever.reach);
    }
    assert!((level - lowest - game.shift_dip).abs() < 1e-3, "dipped {} degrees", degrees(level - lowest));
}

#[test]
fn a_shift_to_a_gear_with_a_lower_cap_brings_the_allowed_angle_down_to_it() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, Ask { gear: 1, ..ask() }, 60 * 8);
    let mut pitch = state.reach;
    for _ in 0..60 * 3 {
        pitch = update(&game, &mut state, &Ask { gear: 3, pitch, front_on_ground: false, ..ask() }).reach;
    }
    assert!((degrees(state.reach) - 30.0).abs() < 0.1, "{} degrees", degrees(state.reach));
}

#[test]
fn the_engine_is_cut_when_the_nose_is_over_the_allowed_angle() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, ask(), 60 * 2);
    let reach = state.reach;
    let at = |pitch: f32| {
        let mut copy = state;
        update(&game, &mut copy, &Ask { pitch, front_on_ground: false, ..ask() }).cut
    };
    assert_eq!(at(reach), 0.0);
    assert_eq!(at(reach + game.cut_over[0] * 0.5), 0.0);
    let some = at(reach + 0.5 * (game.cut_over[0] + game.cut_over[1]));
    assert!(some > 0.0 && some < game.cut_most, "{some}");
    assert!((at(reach + 0.5) - game.cut_most).abs() < 1e-6);
}

/// Put a wheelie up in first gear and its front wheel down again: the state after.
fn put_down() -> State {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, ask(), 60 * 3);
    let up = Ask { pitch: 0.4, front_on_ground: false, ..ask() };
    assert!(update(&game, &mut state, &up).active);
    // The front wheel touches the road with the gas still down.
    assert!(!update(&game, &mut state, &Ask { pitch: 0.0, front_on_ground: true, ..ask() }).active);
    state
}

#[test]
fn a_front_wheel_put_down_is_not_lifted_again_by_the_gas() {
    let game = ThrottleWheelie::GAME;
    let mut state = put_down();
    assert!(state.latched);
    for _ in 0..60 {
        assert!(!update(&game, &mut state, &ask()).active, "latched");
    }
    // In second gear and third, still latched.
    for gear in [2, 3] {
        assert!(!update(&game, &mut state, &Ask { gear, ..ask() }).active);
    }
}

#[test]
fn the_latch_lets_go_in_first_gear_and_at_a_stop() {
    let game = ThrottleWheelie::GAME;
    // Back in first gear (from the second): asked again.
    let mut state = put_down();
    assert!(!update(&game, &mut state, &Ask { gear: 2, ..ask() }).active);
    assert!(update(&game, &mut state, &ask()).active, "first gear again");
    // Stopped (speed 0), in whatever gear that is.
    let mut state = put_down();
    assert!(!update(&game, &mut state, &Ask { speed: 6.0, ..ask() }).active);
    assert!(update(&game, &mut state, &Ask { speed: 0.2, ..ask() }).active, "the bike stopped");
    // Staying in first gear at speed does not release it.
    let mut state = put_down();
    assert!(!update(&game, &mut state, &ask()).active && !update(&game, &mut state, &ask()).active);
}

#[test]
fn a_wheelie_that_never_went_up_is_not_a_wheelie_put_down() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    assert!(update(&game, &mut state, &ask()).active);
    // The throttle is lifted before the nose rose: nothing was lifted, nothing is latched.
    assert!(!update(&game, &mut state, &Ask { throttle: 0.0, ..ask() }).active);
    assert!(!state.latched);
    assert!(update(&game, &mut state, &ask()).active);
}

#[test]
fn blips_of_the_throttle_keep_a_wheelie_up_and_the_latch_waits_for_the_touchdown() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, ask(), 60 * 3);
    let up = Ask { pitch: 0.4, front_on_ground: false, ..ask() };
    for step in 0..120 {
        let throttle = f32::from(step % 20 < 8);
        assert!(update(&game, &mut state, &Ask { throttle, ..up }).active);
    }
    assert!(!state.latched);
}

#[test]
fn the_reference_values_give_the_reference_behaviour() {
    let off = ThrottleWheelie::REFERENCE;
    assert!(!off.enabled);
    for weight in [-1.0, -0.3, 0.0, 0.4] {
        let mut state = State { asked: true, lifted: true, ..State::default() };
        let held = Ask { weight, pitch: 0.5, front_on_ground: false, ..ask() };
        let lever = update(&off, &mut state, &held);
        assert_eq!((lever.weight, lever.active, lever.cut), (weight, false, 0.0));
        assert!(lever.reach.is_infinite());
        assert_eq!(state, State::default());
    }
}

#[test]
fn the_game_caps_are_forty_thirty_five_and_thirty_degrees_in_three_gears() {
    let game = ThrottleWheelie::GAME;
    let caps = [1, 2, 3, 4].map(|gear| game.cap(gear).map(degrees));
    assert!((caps[0].unwrap() - 40.0).abs() < 0.1);
    assert!((caps[1].unwrap() - 35.0).abs() < 0.1);
    assert!((caps[2].unwrap() - 30.0).abs() < 0.1);
    assert_eq!(caps[3], None);
    assert_eq!(game.cap(0), None);
    assert_eq!(ThrottleWheelie::REFERENCE.cap(1), None);
}

#[test]
fn the_clutch_cuts_the_gas_for_the_change_and_the_dip_after_it() {
    let game = ThrottleWheelie::GAME;
    let mut state = State::default();
    ride(&game, &mut state, ask(), 60 * 2);
    let nose = state.reach;
    let quiet = update(&game, &mut state, &Ask { pitch: nose, front_on_ground: false, ..ask() });
    assert_eq!(quiet.cut, 0.0);
    let mut cuts = Vec::new();
    for step in 0..40 {
        let lever =
            update(&game, &mut state, &Ask { shifting: step < 4, pitch: nose, front_on_ground: false, ..ask() });
        cuts.push(lever.cut);
    }
    assert!(cuts[..16].iter().all(|&cut| (cut - game.shift_cut).abs() < 1e-6), "the gas is cut for the dip: {cuts:?}");
    assert_eq!(ThrottleWheelie::REFERENCE.shift_cut, 0.0);
}
