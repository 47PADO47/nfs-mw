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
    // Round 4 ("going faster than 100 kph turn way more"): about 30 m at 80 and at 100 km/h, 45 at 120, 70 at 150, 100 at 180 and 190 at 250.
    for (kmh, metres) in
        [(80.0_f32, 30.0_f32), (100.0, 30.0), (120.0, 45.0), (150.0, 70.0), (180.0, 100.0), (250.0, 190.0)]
    {
        assert!((at(kmh) - metres).abs() < 0.12 * metres, "{kmh} km/h: {} m for {metres}", at(kmh));
    }
}

/// The table of round 3, before the fast turns were tightened.
const ROUND3: &[(f32, f32)] =
    &[(6.0, 1.8), (15.0, 1.9), (30.0, 4.4), (50.0, 14.0), (80.0, 40.0), (120.0, 91.0), (180.0, 240.0)];

#[test]
fn above_100_kmh_the_circle_is_about_half_of_round_3s_or_less_and_the_slow_end_is_kept() {
    let old = TurnLaw { radius: ROUND3, ..law() };
    for kmh in [6.0_f32, 15.0, 30.0, 50.0] {
        let speed = kmh / 3.6;
        assert!((law().radius(speed) - old.radius(speed)).abs() < 1e-3, "{kmh} km/h is kept");
    }
    for kmh in [100.0_f32, 120.0, 150.0, 180.0, 250.0] {
        let speed = kmh / 3.6;
        let (new, before) = (law().radius(speed), old.radius(speed));
        assert!(new <= 0.55 * before, "{kmh} km/h: {new} m against {before} m");
    }
    // 80 km/h is tighter by a third, not by a half: it is not "faster than 100".
    assert!(law().radius(80.0 / 3.6) < 0.75 * old.radius(80.0 / 3.6));
}

#[test]
fn the_progression_is_smooth_and_the_lateral_acceleration_levels_off_above_100_kmh() {
    let law = law();
    // The exponent between neighbouring points of the table is the same few tenths on both sides of every point above 100 km/h: no kink.
    for pair in RADIUS.windows(3).filter(|w| w[0].0 >= 100.0) {
        let exponent = |a: (f32, f32), b: (f32, f32)| (b.1 / a.1).ln() / (b.0 / a.0).ln();
        let (before, after) = (exponent(pair[0], pair[1]), exponent(pair[1], pair[2]));
        assert!((before - after).abs() < 0.35, "{pair:?}: exponents {before} and {after}");
        assert!((1.7..2.4).contains(&before), "{pair:?}: exponent {before}");
    }
    // The acceleration `v^2 / r` is 2.4 to 2.7 g everywhere from 100 km/h.
    for kmh in (100..=300).step_by(10) {
        let g = law.full_acceleration(kmh as f32 / 3.6) / 9.81;
        assert!((2.3..2.8).contains(&g), "{kmh} km/h asks {g} g");
    }
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
    let beyond = law.radius(300.0 / 3.6);
    assert!((beyond - r1 * (300.0 / s1).powf(exponent)).abs() < 1e-2 * beyond);
    assert!(beyond > r1);
    assert_eq!(law.radius(1.0 / 3.6), RADIUS[0].1, "below the first point it is held");
    let empty = TurnLaw { radius: &[], ..law };
    assert!(empty.radius(10.0).is_infinite());
    assert_eq!(empty.acceleration(&steady(1.0, 1.0), 10.0), 0.0);
}

#[test]
fn a_steady_lean_leaves_exactly_the_rest_of_the_circles_acceleration_to_the_assist() {
    let law = law();
    for kmh in [15.0_f32, 30.0, 50.0, 80.0, 100.0, 120.0, 150.0, 180.0, 250.0] {
        let speed = kmh / 3.6;
        let input = steady(1.0, 1.0);
        let assist = law.acceleration(&input, speed);
        let lean = 9.81 * input.target_lean.tan();
        // Lean and assist make the circle's acceleration (a lean that already gives more leaves the assist nothing).
        let total = lean + assist;
        let wanted = law.full_acceleration(speed);
        assert!((0.0..=law.assist_cap(speed) + 1e-3).contains(&assist), "{kmh} km/h: assist {assist}");
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
    assert!(slow > 0.0 && slow <= law.assist_cap(3.0) + 1e-3);
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
fn the_circles_acceleration_leaves_the_assist_room_for_the_stick_s_first_instant() {
    // The lean at its limit (36 degrees, 0.73 g) gives part of the circle's acceleration, the assist the rest, at most `max_assist`; the circle itself
    // must not use all of it, or a flip of the stick (where the assist makes up the whole `wanted - expect`) is capped at once.
    let law = law();
    let lean = 36.0_f32.to_radians().tan();
    assert!(law.max_assist >= 2.3 && law.max_assist <= 2.5, "{}", law.max_assist);
    assert!(law.slow_max_assist <= 1.7 + 1e-6, "the cap at a low speed is round 3's");
    for kmh in (6..320).step_by(2) {
        let speed = kmh as f32 / 3.6;
        let g = law.full_acceleration(speed) / 9.81;
        assert!(g - lean < law.assist_cap(speed) / 9.81 - 0.4, "{kmh} km/h asks {g} g, the assist {} g", g - lean);
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
    let cap = law.assist_cap(15.0);
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

#[test]
fn the_most_the_assist_gives_is_round_3s_at_a_low_speed_and_rises_to_2_4_g_above_72_kmh() {
    let law = law();
    assert!((law.assist_cap(0.0) - 1.7 * 9.81).abs() < 1e-4);
    assert!((law.assist_cap(law.max_assist_speed[0]) - 1.7 * 9.81).abs() < 1e-4);
    assert!((law.assist_cap(law.max_assist_speed[1]) - 2.4 * 9.81).abs() < 1e-4);
    assert!((law.assist_cap(100.0) - 2.4 * 9.81).abs() < 1e-4);
    assert!((law.assist_cap(-100.0) - law.assist_cap(100.0)).abs() < 1e-6, "the speed's sign does not matter");
    let mut last = law.assist_cap(0.0);
    for step in 1..400 {
        let now = law.assist_cap(step as f32 * 0.25);
        assert!(now >= last - 1e-5 && now - last < 0.5, "monotonic and without a step");
        last = now;
    }
    // A flip of the stick at 15 km/h is held to the 1.7 g of round 3, at 100 km/h to 2.4 g.
    let flip = TurnInput { target_share: -1.0, stick_share: 1.0, target_lean: -36.0_f32.to_radians() };
    assert!(law.acceleration(&flip, 15.0 / 3.6).abs() <= 1.7 * 9.81 + 1e-3);
    // (At speed the share is the lean target's, so a flip's circle is the old side's; a stick that has just reached full asks for more.)
    let full = TurnInput { target_share: -1.0, stick_share: -1.0, target_lean: 36.0_f32.to_radians() };
    assert!(law.acceleration(&full, 100.0 / 3.6).abs() > 1.7 * 9.81);
}
