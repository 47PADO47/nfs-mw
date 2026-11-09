use super::*;

fn law() -> TurnLaw {
    TurnLaw::GAME
}

fn steady(share: f32, lean_share: f32) -> TurnInput {
    // A bike whose lean has reached the target of `lean_share` of a 36 degree limit.
    TurnInput { target_share: share, stick_share: share, target_lean: lean_share * 36.0_f32.to_radians() }
}

#[test]
fn the_table_is_ascending_in_speed_and_in_radius_and_hits_the_players_numbers() {
    let table = RADIUS;
    assert!(table.windows(2).all(|pair| pair[1].0 > pair[0].0 && pair[1].1 > pair[0].1), "{table:?}");
    assert!(table[0].0 > 0.0 && table[0].1 > 0.0);
    let at = |kmh: f32| law().radius(kmh / 3.6);
    assert!(at(15.0) <= 3.0, "15 km/h: {} m", at(15.0));
    assert!(at(30.0) <= 6.0, "30 km/h: {} m", at(30.0));
    assert!((at(50.0) - 15.0).abs() < 2.0, "50 km/h: {} m", at(50.0));
    assert!((at(80.0) - 40.0).abs() < 5.0, "80 km/h: {} m", at(80.0));
    assert!((at(120.0) - 90.0).abs() < 10.0, "120 km/h: {} m", at(120.0));
    assert!(at(180.0) >= 220.0, "180 km/h: {} m", at(180.0));
}

#[test]
fn the_radius_never_falls_with_the_speed_and_has_no_step() {
    let law = law();
    let mut last = law.radius(0.0);
    let mut speed = 0.0;
    while speed < 120.0 {
        speed += 0.05;
        let now = law.radius(speed);
        assert!(now >= last - 1e-4, "{speed} m/s: {now} after {last}");
        assert!(now < last * 1.02 + 0.01, "a step at {speed} m/s: {last} to {now}");
        last = now;
    }
}

#[test]
fn the_radius_passes_through_the_points_of_the_table_and_grows_by_the_last_exponent_beyond() {
    let law = law();
    for &(kmh, metres) in RADIUS {
        assert!((law.radius(kmh / 3.6) - metres).abs() < 1e-3 * metres, "{kmh} km/h");
    }
    let [(s0, r0), (s1, r1)] = [RADIUS[RADIUS.len() - 2], RADIUS[RADIUS.len() - 1]];
    let exponent = (r1 / r0).ln() / (s1 / s0).ln();
    let beyond = law.radius(250.0 / 3.6);
    assert!((beyond - r1 * (250.0 / s1).powf(exponent)).abs() < 1e-2 * beyond);
    assert!(beyond > r1);
    assert_eq!(law.radius(1.0 / 3.6), RADIUS[0].1, "below the first point it is held");
    let empty = TurnLaw { radius: &[], ..law };
    assert!(empty.radius(10.0).is_infinite());
    assert_eq!(empty.acceleration(&steady(1.0, 1.0), 10.0), 0.0);
}

#[test]
fn a_steady_lean_leaves_exactly_the_rest_of_the_circles_acceleration_to_the_assist() {
    let law = law();
    for kmh in [15.0_f32, 30.0, 50.0, 80.0, 120.0, 180.0] {
        let speed = kmh / 3.6;
        let input = steady(1.0, 1.0);
        let assist = law.acceleration(&input, speed);
        let lean = 9.81 * input.target_lean.tan();
        // Lean and assist make the circle's acceleration (a lean that already gives more leaves the assist nothing).
        let total = lean + assist;
        let wanted = law.full_acceleration(speed);
        assert!((0.0..=law.max_assist * 9.81 + 1e-3).contains(&assist), "{kmh} km/h: assist {assist}");
        assert!((total - wanted).abs() < 1e-3 * wanted || wanted < lean, "{kmh} km/h: {total} against {wanted}");
    }
}

#[test]
fn the_assist_follows_the_stick_and_never_works_against_it() {
    let law = law();
    let right = law.acceleration(&steady(1.0, 0.6), 15.0);
    let left = law.acceleration(&steady(-1.0, -0.6), 15.0);
    assert!(right > 0.0 && (right + left).abs() < 1e-4, "right {right}, left {left}");
    let half = law.acceleration(&steady(0.5, 0.3), 15.0);
    assert!(half > 0.0 && half < right);
    // A bike still leaning over the limit of a small stick has nothing to add: the lean already gives more than the circle's share.
    let over = TurnInput { target_share: 0.05, stick_share: 0.05, target_lean: 36.0_f32.to_radians() };
    assert_eq!(law.acceleration(&over, 15.0), 0.0);
    // No stick, no assist.
    assert_eq!(law.acceleration(&TurnInput::default(), 15.0), 0.0);
    // Against the stick's side (a target lean to the left with a right stick, as in a flip) the whole acceleration is the assist's, to the cap.
    let flip = TurnInput { target_share: -1.0, stick_share: 1.0, target_lean: -36.0_f32.to_radians() };
    let slow = law.acceleration(&flip, 3.0);
    assert!(slow > 0.0 && slow <= law.max_assist * 9.81 + 1e-3);
}

#[test]
fn the_assist_is_faded_out_below_walking_pace() {
    let law = law();
    assert_eq!(law.acceleration(&steady(1.0, 0.0), 0.0), 0.0);
    assert_eq!(law.acceleration(&steady(1.0, 0.0), law.fade[0]), 0.0);
    let walking = law.acceleration(&steady(1.0, 0.0), 0.5 * (law.fade[0] + law.fade[1]));
    let faster = law.acceleration(&steady(1.0, 0.0), law.fade[1] + 1.0);
    assert!(walking > 0.0 && walking < faster);
}

#[test]
fn a_low_speed_path_follows_the_stick_and_a_high_speed_one_the_lean_target() {
    let law = law();
    let input = TurnInput { target_share: 0.1, stick_share: 1.0, target_lean: 0.1 * 36.0_f32.to_radians() };
    assert!((law.share(&input, law.quick[0] - 1.0) - 1.0).abs() < 1e-5);
    assert!((law.share(&input, law.quick[1] + 1.0) - 0.1).abs() < 1e-5);
    let middle = law.share(&input, 0.5 * (law.quick[0] + law.quick[1]));
    assert!((middle - 0.55).abs() < 1e-5, "{middle}");
    // At a slow speed the first instant of a full stick already turns, with the lean target still at 0.1.
    assert!(law.acceleration(&input, 3.0) > 0.0);
    let later = law.acceleration(&TurnInput { target_share: 1.0, ..input }, 3.0);
    assert!(law.acceleration(&input, 3.0) >= 0.5 * later);
}

#[test]
fn the_lean_in_is_quicker_at_a_low_speed_and_unchanged_at_a_high_one() {
    let law = law();
    assert!((law.lean_rate(0.0) - law.slow_lean_rate).abs() < 1e-5);
    assert_eq!(law.lean_rate(law.slow_lean_speed[1]), 1.0);
    assert_eq!(law.lean_rate(60.0), 1.0);
    let mut last = law.lean_rate(0.0);
    for step in 1..200 {
        let now = law.lean_rate(step as f32 * 0.1);
        assert!(now <= last + 1e-6);
        last = now;
    }
    assert_eq!(law.lean_rate(-law.slow_lean_speed[1]), 1.0, "the speed's sign does not matter");
}

#[test]
fn the_circles_acceleration_stays_under_what_the_earlier_rounds_turned_at() {
    let law = law();
    for kmh in (6..260).step_by(2) {
        let speed = kmh as f32 / 3.6;
        let g = law.full_acceleration(speed) / 9.81;
        assert!(g < 1.7, "{kmh} km/h asks {g} g");
    }
}

#[test]
fn the_trim_grows_while_the_bike_turns_less_than_the_circle_and_settles_when_it_turns_the_circle() {
    let law = law();
    let input = steady(1.0, 1.0);
    let speed = 15.0;
    let wanted = law.wanted(&input, speed);
    let mut trim = 0.0;
    for _ in 0..60 {
        trim = law.trim_step(trim, &input, speed, 0.6 * wanted, 1.0, 1.0 / 60.0);
    }
    assert!(trim > 1.0, "a turn at 60 percent of the circle grows the trim: {trim}");
    let grown = trim;
    for _ in 0..600 {
        trim = law.trim_step(trim, &input, speed, wanted, 1.0, 1.0 / 60.0);
    }
    assert!((trim - grown).abs() < 1e-3, "the circle itself leaves it where it is: {trim} from {grown}");
    for _ in 0..600 {
        trim = law.trim_step(trim, &input, speed, 1.3 * wanted, 1.0, 1.0 / 60.0);
    }
    assert!(trim < grown - 1.0, "a turn over the circle takes it back: {trim}");
    let ff = law.acceleration(&input, speed).abs();
    assert!(trim >= -ff - 1e-4, "never takes more than the assist has: {trim}");
}

#[test]
fn the_trim_is_held_by_the_cap_dies_away_with_the_stick_and_the_speed_and_is_not_wound_up_by_a_flip() {
    let law = law();
    let input = steady(1.0, 1.0);
    let cap = law.max_assist * 9.81;
    let mut trim = 0.0;
    for _ in 0..6000 {
        trim = law.trim_step(trim, &input, 15.0, -5.0, 1.0, 1.0 / 60.0);
    }
    let total = law.trimmed(&input, 15.0, trim).abs();
    assert!(total <= cap + 1e-3 && trim + law.acceleration(&input, 15.0).abs() <= cap + 1e-3, "{trim} {total}");
    for _ in 0..600 {
        trim = law.trim_step(trim, &TurnInput::default(), 15.0, 0.0, 1.0, 1.0 / 60.0);
    }
    assert!(trim.abs() < 1e-2, "no stick: {trim}");
    assert_eq!(law.trim_step(5.0, &input, 0.0, 0.0, 1.0, 1.0 / 60.0), 0.0, "no assist at a standstill, no trim");
    // The rate the shortfall counts at is bounded: one step moves the trim by `gain x reach x dt` at most.
    let step = law.trim_step(0.0, &input, 15.0, -40.0, 1.0, 1.0 / 60.0);
    assert!(step <= law.trim_gain * law.trim_reach / 60.0 + 1e-5, "{step}");
    // With a surface that takes the grip away the trim wants less.
    let loose = law.trim_step(0.0, &input, 15.0, 0.0, 0.3, 1.0 / 60.0);
    assert!(loose < law.trim_step(0.0, &input, 15.0, 0.0, 1.0, 1.0 / 60.0) + 1e-6);
    // The trimmed assist keeps the stick's side.
    assert!(law.trimmed(&steady(-1.0, -1.0), 15.0, 2.0) < 0.0 && law.trimmed(&input, 15.0, 2.0) > 0.0);
}
