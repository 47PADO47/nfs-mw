use super::super::style::StandLeg;
use super::*;

const DT: f32 = 1.0 / 60.0;

fn leg(left_ik: Option<[f32; 4]>, right_ik: Option<[f32; 4]>) -> StandLeg {
    StandLeg {
        start_seconds: [0.0, 2.0],
        stop_seconds: [1.0, 3.0],
        start_rate: 1.0,
        stop_rate: 1.0,
        left_ik,
        right_ik,
        ik_rate: [4.0; 2],
    }
}

fn stand() -> Stand {
    let ik = Some([0.1, 1.0, 1.5, 2.0]);
    Stand { speed_kmh: 6.0, racing_at: 0.85, legs: [leg(ik, ik), leg(ik, None), leg(None, ik)] }
}

fn run(standstill: &mut Standstill, speed_kmh: f32, seconds: f32) -> Signals {
    let mut out = Signals::default();
    for _ in 0..(seconds / DT).round() as usize {
        out = standstill.step(speed_kmh, DT);
    }
    out
}

#[test]
fn a_bike_that_starts_standing_waits_with_its_feet_on_the_ground() {
    let mut standstill = Standstill::new(Some(stand()), Leg::Both);
    let out = run(&mut standstill, 0.0, 0.5);
    assert_eq!((out.weight, out.state), (1.0, 1.0), "{out:?}");
    assert!(out.pegs.iter().all(|peg| *peg < 1e-3), "both feet are off their pegs: {out:?}");
    assert_eq!(out.stop_time, 3.0, "the stop clip is at the end of its window");
}

#[test]
fn a_bike_that_starts_moving_rides_with_its_feet_on_the_pegs() {
    let mut standstill = Standstill::new(Some(stand()), Leg::Both);
    let out = run(&mut standstill, 30.0, 0.5);
    assert_eq!((out.weight, out.pegs), (0.0, [1.0, 1.0]), "{out:?}");
}

#[test]
fn pulling_away_plays_the_start_clip_and_puts_the_feet_back_on_the_pegs() {
    let mut standstill = Standstill::new(Some(stand()), Leg::Both);
    run(&mut standstill, 0.0, 0.5);
    // Just above the threshold: the start clip begins at the first second of its window.
    let early = run(&mut standstill, 10.0, 0.2);
    assert_eq!(early.state, 2.0, "{early:?}");
    assert!(early.start_time > 0.3 && early.start_time < 0.5, "{early:?}");
    assert!(early.weight > 0.9 && early.pegs.iter().all(|peg| *peg < 1e-3), "{early:?}");
    // Past the second `on Footboard at:` the feet are held again; the start value passes 0.85 and the riding pose takes over.
    let late = run(&mut standstill, 10.0, 2.0);
    assert_eq!((late.weight, late.pegs), (0.0, [1.0, 1.0]), "{late:?}");
}

#[test]
fn stopping_plays_the_stop_clip_and_takes_the_feet_off_the_pegs_inside_it() {
    let mut standstill = Standstill::new(Some(stand()), Leg::Both);
    run(&mut standstill, 30.0, 1.0);
    let first = run(&mut standstill, 0.0, 0.1);
    assert_eq!(first.state, 0.0, "the stop clip: {first:?}");
    assert!(first.stop_time > 1.0 && first.stop_time < 1.4, "{first:?}");
    assert_eq!(first.pegs, [1.0, 1.0], "the feet are on the pegs until the clip's off-footboard second");
    let done = run(&mut standstill, 0.0, 4.0);
    assert_eq!((done.state, done.weight), (1.0, 1.0), "{done:?}");
    assert!(done.pegs.iter().all(|peg| *peg < 1e-3), "both feet are off their pegs: {done:?}");
}

#[test]
fn putting_one_foot_down_leaves_the_other_on_its_peg() {
    let mut right = Standstill::new(Some(stand()), Leg::Right);
    let out = run(&mut right, 0.0, 0.5);
    assert!(
        out.leg == 2.0 && out.pegs[0] == 1.0 && out.pegs[1] < 1e-3,
        "the right foot is down, the left stays up: {out:?}"
    );
    let mut left = Standstill::new(Some(stand()), Leg::Left);
    let out = run(&mut left, 0.0, 0.5);
    assert!(out.leg == 1.0 && out.pegs[0] < 1e-3 && out.pegs[1] == 1.0, "{out:?}");
}

#[test]
fn a_rider_without_a_standstill_never_puts_his_feet_down() {
    let mut standstill = Standstill::new(None, Leg::Both);
    assert_eq!(run(&mut standstill, 0.0, 1.0), Signals { pegs: [1.0, 1.0], ..Signals::default() });
}

#[test]
fn stopping_again_before_the_stop_clip_ends_goes_back_to_riding() {
    let mut standstill = Standstill::new(Some(stand()), Leg::Both);
    run(&mut standstill, 30.0, 1.0);
    let stopping = run(&mut standstill, 0.0, 0.3);
    assert_eq!(stopping.state, 0.0);
    let again = run(&mut standstill, 30.0, 1.5);
    assert_eq!((again.weight, again.pegs), (0.0, [1.0, 1.0]), "{again:?}");
}
