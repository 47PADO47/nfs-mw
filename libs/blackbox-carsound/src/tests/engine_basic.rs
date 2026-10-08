//! Steady driving: idle, launch, redline, decel, the frequency mapping and the output contract.

use super::common::{all_finite, hold, input, run, tuning};
use crate::{CarInput, EngineMixer, TICK_SECONDS};

const SLEW: f32 = 7000.0 / 32767.0;

fn mixer() -> EngineMixer {
    EngineMixer::new(&tuning())
}

#[test]
fn idle_sits_at_the_minimum_frequency_and_stays_quiet_on_the_decel_loop() {
    let mut m = mixer();
    let out = *hold(&mut m, 2.0, input(0.0, 0.0, 1)).last().unwrap();
    assert!((out.eng_rpm - 1000.0).abs() < 1.0, "{}", out.eng_rpm);
    assert!((out.ginsu_frequency - 1500.0).abs() < 1.0);
    assert_eq!(out.decel_volume, 0.0, "the decel window starts above idle");
    assert!(out.accel_volume > 0.0 && out.accel_volume < 1.0);
    assert!(!out.redlining);
    assert!(all_finite(&out));
}

#[test]
fn ginsu_frequency_follows_the_physics_rpm_on_the_sound_scale() {
    let mut m = mixer();
    // rpm_pct 0.5 on the straight map: 1000 + 9000 * 0.5 = 5500 sound rpm.
    let out = *hold(&mut m, 3.0, input(0.5, 0.2, 4)).last().unwrap();
    assert!((out.physics_rpm - 5500.0).abs() < 1.0);
    let expected = (5500.0 - 1000.0) / 9000.0 * (7784.0 - 1500.0) + 1500.0;
    assert!((out.ginsu_frequency - expected).abs() < 5.0, "{} vs {expected}", out.ginsu_frequency);
}

#[test]
fn the_rpm_map_bends_the_physics_rpm_of_the_local_player_only() {
    let mut t = tuning();
    t.engine.rpm_map = [0.0, 0.4136, 0.6893, 1.0];
    let bent = 0.375 * 0.4136 + 0.375 * 0.6893 + 0.125;
    let mut player = EngineMixer::new(&t);
    let out = *hold(&mut player, 1.0, input(0.5, 0.0, 4)).last().unwrap();
    assert!((out.physics_rpm - (1000.0 + 9000.0 * bent)).abs() < 1.0, "{}", out.physics_rpm);
    t.is_local_player = false;
    let mut other = EngineMixer::new(&t);
    let out = *hold(&mut other, 1.0, input(0.5, 0.0, 4)).last().unwrap();
    assert!((out.physics_rpm - 5500.0).abs() < 1.0);
}

#[test]
fn launch_revs_above_the_physics_and_volumes_rise_by_at_most_the_slew() {
    let mut m = mixer();
    let outs = run(&mut m, 120, |i| input((i as f32 / 90.0).min(0.6) * 0.8, 1.0, 2));
    // The stab from idle revs past the physics for a while (peak 3000 over 300 ms).
    let revved = outs.iter().take(30).any(|o| o.eng_rpm > o.physics_rpm + 500.0);
    assert!(revved, "no idle rev: {:?}", outs.iter().take(30).map(|o| o.eng_rpm).collect::<Vec<_>>());
    for pair in outs.windows(2) {
        let up = pair[1].accel_volume - pair[0].accel_volume;
        assert!(up.abs() <= SLEW + 1e-4, "step {up}");
    }
    assert!(outs.last().unwrap().accel_volume > 0.5);
}

#[test]
fn full_throttle_at_the_limiter_ducks_the_engine_and_raises_the_redline_sample() {
    let mut m = mixer();
    let before = *hold(&mut m, 1.0, input(0.9, 1.0, 3)).last().unwrap();
    assert!(!before.redlining);
    assert_eq!(before.redline_sample_volume, 0.0);
    let after = *hold(&mut m, 2.5, input(1.0, 1.0, 3)).last().unwrap();
    assert!(after.redlining, "eng rpm {}", after.eng_rpm);
    assert!(after.accel_volume < before.accel_volume * 0.3, "{} vs {}", after.accel_volume, before.accel_volume);
    assert!(after.redline_sample_volume > 0.5, "{}", after.redline_sample_volume);
    let back = *hold(&mut m, 1.0, input(0.5, 1.0, 3)).last().unwrap();
    assert!(!back.redlining);
    assert!(back.redline_sample_volume < 0.05);
    assert!(back.accel_volume > after.accel_volume * 2.0);
}

#[test]
fn lifting_off_moves_the_sound_from_the_accelerate_to_the_decelerate_loop() {
    let mut m = mixer();
    let on = *hold(&mut m, 2.0, input(0.55, 1.0, 4)).last().unwrap();
    let off = *hold(&mut m, 2.0, input(0.55, 0.0, 4)).last().unwrap();
    assert!(off.decel_volume > 0.3, "{}", off.decel_volume);
    assert!(off.decel_volume > on.decel_volume);
    assert!(off.accel_volume < on.accel_volume);
}

#[test]
fn the_decel_loop_stays_silent_outside_its_rpm_window() {
    let mut m = mixer();
    // Window is 2000..7000 on the Ginsu axis; rpm_pct 1.0 without redline is at 7784, above it.
    let mut t = tuning();
    t.engine.redline_enabled = false;
    let mut m2 = EngineMixer::new(&t);
    let out = *hold(&mut m2, 2.0, input(1.0, 0.0, 4)).last().unwrap();
    assert_eq!(out.decel_volume, 0.0);
    let low = *hold(&mut m, 2.0, input(0.02, 0.0, 4)).last().unwrap();
    assert_eq!(low.decel_volume, 0.0);
}

#[test]
fn single_mode_never_uses_the_decelerate_loop() {
    let mut t = tuning();
    t.engine.mode = crate::EngineMode::Single;
    let mut m = EngineMixer::new(&t);
    let out = *hold(&mut m, 2.0, input(0.5, 0.0, 4)).last().unwrap();
    assert_eq!(out.decel_volume, 0.0);
    assert!(out.accel_volume > 0.0);
}

#[test]
fn loops_below_their_minimum_play_at_the_minimum_with_the_ratio_as_playback_rate() {
    let mut t = tuning();
    t.engine.accel_loop_min_frequency = 1800.0;
    t.engine.decel_loop_min_frequency = 0.0;
    let mut m = EngineMixer::new(&t);
    let out = *hold(&mut m, 1.0, CarInput { pitch_multiplier: 2.0, ..input(0.0, 0.0, 1) }).last().unwrap();
    assert_eq!(out.accel_loop.frequency, 1800.0);
    assert!((out.accel_loop.playback_rate - 1500.0 / 1800.0 * 2.0).abs() < 1e-3);
    assert_eq!(out.decel_loop.frequency, out.ginsu_frequency);
    assert!((out.decel_loop.playback_rate - 2.0).abs() < 1e-6);
    assert_eq!(out.playback_rate, out.accel_loop.playback_rate);
}

#[test]
fn the_tachometer_shows_the_physics_before_the_race_and_the_audio_after_it() {
    // Revs held high from a standing start: the audio RPM climbs at most 999 per tick (clutch model).
    let pre = CarInput { pre_race: true, ..input(0.9, 0.0, 1) };
    let mut m = mixer();
    let out = m.update(TICK_SECONDS, &pre);
    assert!((out.visual_rpm_fraction - 0.9).abs() < 1e-3);
    assert!(out.eng_rpm < 2100.0, "the audio lags: {}", out.eng_rpm);

    // Another car's gauge is always the audio value.
    let mut t = tuning();
    t.is_local_player = false;
    let mut other = EngineMixer::new(&t);
    let out = other.update(TICK_SECONDS, &pre);
    assert!((out.visual_rpm_fraction - 0.9).abs() > 0.3, "the gauge is the audio value: {}", out.visual_rpm_fraction);

    // After the countdown the gauge follows the audio: through an up shift it glides from the RPM at the
    // shift down to the new physics value instead of jumping.
    let mut m = mixer();
    hold(&mut m, 2.0, CarInput { pre_race: true, ..input(0.9, 1.0, 2) });
    let racing = CarInput { pre_race: false, ..input(0.6, 1.0, 3) };
    let outs = run(&mut m, 90, |_| racing);
    assert!(outs[0].visual_rpm_fraction > 0.8, "starts high: {}", outs[0].visual_rpm_fraction);
    let mid = outs[20].visual_rpm_fraction;
    assert!(mid < outs[0].visual_rpm_fraction && mid > 0.6, "gliding: {mid}");
    assert!((outs[89].visual_rpm_fraction - 0.6).abs() < 0.03, "{}", outs[89].visual_rpm_fraction);
}

#[test]
fn compression_bumps_appear_in_steady_cruising_only() {
    let mut m = mixer();
    let cruise = CarInput { speed: 30.0, ..input(0.5, 0.2, 5) };
    let outs = hold(&mut m, 12.0, cruise);
    let bumps = outs.iter().filter(|o| o.eng_rpm > 5500.0 + 20.0).count();
    assert!(bumps > 0, "no compression bump in 12 s of cruising");
    let mut slow = mixer();
    let outs = hold(&mut slow, 12.0, CarInput { speed: 5.0, ..input(0.5, 0.2, 5) });
    assert!(outs.iter().skip(60).all(|o| (o.eng_rpm - 5500.0).abs() < 20.0));
}

#[test]
fn equal_inputs_give_bit_identical_outputs() {
    let script =
        |i: usize| input(((i as f32) * 0.013).sin().abs(), ((i as f32) * 0.031).cos().abs(), 2 + (i / 90) as i32 % 4);
    let a = run(&mut mixer(), 600, script);
    let b = run(&mut mixer(), 600, script);
    assert_eq!(a, b);
}

#[test]
fn frame_time_does_not_change_the_result_at_the_tick_rate() {
    let script = |i: usize| input(((i as f32) * 0.02).sin().abs(), 1.0, 3);
    let mut by_tick = mixer();
    let mut by_frame = mixer();
    let mut last = None;
    for i in 0..300 {
        by_tick.update(TICK_SECONDS, &script(i));
        // Two half-tick frames carry the same telemetry as one tick.
        by_frame.update(TICK_SECONDS / 2.0, &script(i));
        last = Some(by_frame.update(TICK_SECONDS / 2.0, &script(i)));
    }
    assert_eq!(last.unwrap().ginsu_frequency, by_tick.output().ginsu_frequency);
}
