//! Shifting and the throttle-stab rev.

use super::common::{all_finite, hold, input, run, tuning};
use crate::{EngineMixer, EngineOutput, ShiftDirection, ShiftState};

fn mixer() -> EngineMixer {
    EngineMixer::new(&tuning())
}

/// The distinct shift states in the order they first appear.
fn states(outs: &[EngineOutput]) -> Vec<ShiftState> {
    let mut seen = Vec::new();
    for out in outs {
        if out.shift_state != ShiftState::None && seen.last() != Some(&out.shift_state) {
            seen.push(out.shift_state);
        }
    }
    seen
}

fn clunks(outs: &[EngineOutput]) -> Vec<ShiftDirection> {
    outs.iter().filter_map(|o| o.events.gear_clunk).collect()
}

#[test]
fn an_up_shift_drops_the_rpm_engages_wobbles_and_ends() {
    let mut m = mixer();
    hold(&mut m, 1.0, input(0.8, 1.0, 2));
    let before = *hold(&mut m, 0.5, input(0.8, 1.0, 2)).last().unwrap();
    let outs = run(&mut m, 120, |_| input(0.55, 1.0, 3));
    assert_eq!(
        states(&outs),
        vec![ShiftState::UpDisengage, ShiftState::UpEngaging, ShiftState::UpLfo],
        "the stages of an up shift"
    );
    assert_eq!(outs.last().unwrap().shift_state, ShiftState::None);
    let first = outs[0];
    assert_eq!(first.shift_state, ShiftState::UpDisengage);
    assert!(first.events.disengage_sweetener);
    assert!((first.rpm_at_shift - before.eng_rpm).abs() < 5.0);
    // The clutch is out: the audio RPM falls by about 3500 from the RPM at the shift.
    let low = outs.iter().map(|o| o.eng_rpm).fold(f32::MAX, f32::min);
    assert!(low < first.rpm_at_shift - 3000.0, "low {low}, at shift {}", first.rpm_at_shift);
    assert!(low >= 1000.0 - 1.0);
    assert!(outs.iter().all(all_finite));
    // The engage overshoots above the physics (curve peak 600) and the wobble settles around it.
    let physics = outs[0].physics_rpm;
    assert!(outs.iter().any(|o| o.shift_state == ShiftState::UpEngaging && o.eng_rpm > physics + 100.0));
    let end = outs.last().unwrap();
    assert!((end.eng_rpm - physics).abs() < 130.0, "{} vs {physics}", end.eng_rpm);
}

#[test]
fn an_up_shift_plays_one_clunk_after_its_delay_and_one_engage_sweetener() {
    let mut m = mixer();
    hold(&mut m, 1.0, input(0.85, 1.0, 2));
    let outs = run(&mut m, 90, |_| input(0.6, 1.0, 3));
    assert_eq!(clunks(&outs), vec![ShiftDirection::Up]);
    let at = outs.iter().position(|o| o.events.gear_clunk.is_some()).unwrap();
    // Up_Shift_Sound_Delay is 50 ms: the clunk is due after about 3 ticks, not on the first one.
    assert!((2..=5).contains(&at), "tick {at}");
    assert_eq!(outs.iter().filter(|o| o.events.engage_sweetener).count(), 1);
    assert_eq!(outs.iter().filter(|o| o.events.disengage_sweetener).count(), 1);
}

#[test]
fn the_engage_of_an_up_shift_raises_the_engine_volume() {
    let mut m = mixer();
    hold(&mut m, 1.5, input(0.7, 1.0, 2));
    let steady = m.output().accel_volume;
    let outs = run(&mut m, 60, |_| input(0.7, 1.0, 3));
    let peak = outs.iter().map(|o| o.accel_volume).fold(0.0, f32::max);
    assert!(peak > steady, "peak {peak} vs steady {steady}");
}

#[test]
fn shifting_up_at_idle_or_with_no_pattern_does_nothing() {
    let mut m = mixer();
    hold(&mut m, 0.5, input(0.0, 0.0, 1));
    let outs = run(&mut m, 60, |_| input(0.0, 0.5, 2));
    assert!(states(&outs).is_empty(), "an up shift below 3000 rpm is ignored");

    let mut t = tuning();
    t.shift.up_engage = None;
    let mut bare = EngineMixer::new(&t);
    hold(&mut bare, 0.5, input(0.8, 1.0, 2));
    let outs = run(&mut bare, 60, |_| input(0.5, 1.0, 3));
    assert!(states(&outs).is_empty());
    assert!(outs.iter().all(all_finite));
}

#[test]
fn a_car_that_starts_in_gear_does_not_shift() {
    let mut m = mixer();
    let outs = run(&mut m, 60, |_| input(0.7, 1.0, 4));
    assert!(states(&outs).is_empty());
    assert!(clunks(&outs).is_empty());
}

#[test]
fn a_down_shift_blips_the_throttle_and_reattaches() {
    let mut m = mixer();
    hold(&mut m, 1.5, input(0.6, 0.0, 4));
    let outs = run(&mut m, 150, |_| input(0.75, 0.0, 3));
    assert_eq!(
        states(&outs),
        vec![
            ShiftState::DownDisengage,
            ShiftState::DownEngagingRise,
            ShiftState::DownEngagingFall,
            ShiftState::DownEngagingReattach
        ]
    );
    assert_eq!(outs.last().unwrap().shift_state, ShiftState::None);
    assert_eq!(clunks(&outs), vec![ShiftDirection::Down]);
    let physics = outs[0].physics_rpm;
    let rise = outs.iter().filter(|o| o.shift_state == ShiftState::DownEngagingRise);
    assert!(rise.map(|o| o.eng_rpm).fold(0.0, f32::max) > physics + 600.0, "the rev-match blip overshoots");
    assert!((outs.last().unwrap().eng_rpm - physics).abs() < 130.0);
}

#[test]
fn a_down_shift_into_neutral_is_ignored() {
    let mut m = mixer();
    hold(&mut m, 1.0, input(0.6, 0.0, 3));
    let outs = run(&mut m, 30, |_| input(0.6, 0.0, 1));
    assert!(states(&outs).is_empty());
}

#[test]
fn gear_changes_while_shifting_restart_the_shift_cleanly() {
    let mut m = mixer();
    hold(&mut m, 1.0, input(0.8, 1.0, 2));
    let mut outs = run(&mut m, 6, |_| input(0.75, 1.0, 3));
    assert_eq!(outs.last().unwrap().shift_state, ShiftState::UpDisengage);
    // Up again 100 ms later, then straight down, then up: nothing may panic or go non-finite.
    outs.extend(run(&mut m, 6, |_| input(0.7, 1.0, 4)));
    assert_eq!(outs.last().unwrap().shift_state, ShiftState::UpDisengage);
    outs.extend(run(&mut m, 6, |_| input(0.7, 0.0, 3)));
    assert_eq!(outs.last().unwrap().shift_state, ShiftState::DownDisengage);
    outs.extend(run(&mut m, 6, |_| input(0.7, 1.0, 4)));
    outs.extend(run(&mut m, 240, |_| input(0.6, 1.0, 4)));
    assert_eq!(outs.last().unwrap().shift_state, ShiftState::None);
    assert!(outs.iter().all(all_finite));
}

#[test]
fn a_throttle_stab_at_speed_attacks_above_the_physics_rpm_and_plays_the_sweetener() {
    let mut m = mixer();
    hold(&mut m, 3.0, input(0.5, 0.0, 4));
    let outs = run(&mut m, 60, |_| input(0.5, 1.0, 4));
    assert!(outs[0].events.accel_sweetener);
    assert!(outs[0].eng_rpm > outs[0].physics_rpm + 500.0, "{} vs {}", outs[0].eng_rpm, outs[0].physics_rpm);
    // 500 ms later it has settled back onto the physics.
    assert!((outs[40].eng_rpm - outs[40].physics_rpm).abs() < 150.0);
    // A second stab within 2 s makes no attack.
    hold(&mut m, 0.3, input(0.5, 0.0, 4));
    let again = run(&mut m, 30, |_| input(0.5, 1.0, 4));
    assert!(again.iter().all(|o| !o.events.accel_sweetener));
}

#[test]
fn releasing_the_throttle_at_high_rpm_plays_the_engine_off_sweetener() {
    let mut m = mixer();
    hold(&mut m, 3.0, input(0.8, 1.0, 4));
    let outs = run(&mut m, 30, |_| input(0.8, 0.0, 4));
    assert_eq!(outs.iter().filter(|o| o.events.engine_off_sweetener).count(), 1);
    let mut low = mixer();
    hold(&mut low, 3.0, input(0.3, 1.0, 4));
    let outs = run(&mut low, 30, |_| input(0.3, 0.0, 4));
    assert!(outs.iter().all(|o| !o.events.engine_off_sweetener));
}

#[test]
fn the_idle_rev_pulls_the_audio_rpm_up_then_settles_and_a_release_interrupts_it() {
    let mut m = mixer();
    hold(&mut m, 1.0, input(0.0, 0.0, 2));
    let outs = run(&mut m, 120, |_| input(0.0, 1.0, 2));
    let peak = outs.iter().map(|o| o.eng_rpm).fold(0.0, f32::max);
    assert!(peak > 3000.0, "peak {peak}");
    assert!((outs.last().unwrap().eng_rpm - 1000.0).abs() < 100.0, "{}", outs.last().unwrap().eng_rpm);

    // Releasing mid-rev returns to the physics over the interrupt time.
    let mut m = mixer();
    hold(&mut m, 1.0, input(0.0, 0.0, 2));
    run(&mut m, 12, |_| input(0.0, 1.0, 2));
    let outs = run(&mut m, 90, |_| input(0.0, 0.0, 2));
    assert!((outs.last().unwrap().eng_rpm - 1000.0).abs() < 100.0);
}
