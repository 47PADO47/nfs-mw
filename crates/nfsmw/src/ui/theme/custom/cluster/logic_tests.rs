//! Tests for the cluster's logic that needs no drawing: the spring and the pulses, the memory between frames, the rev
//! bands and the shift cue, the units, the bars' fills and the advanced row's fields. They run on a `HudState` set by
//! hand, with no game data and no GPU.

use std::sync::Arc;

use super::super::feed::CustomFeed;
use super::super::tokens::Motion;
use super::advanced::{Field, fields, load_percent, rpm_text};
use super::bars::boost_fill;
use super::memory::{Memory, Pulse, SPEEDBREAKER_HIDE_S, Spring};
use super::revs::{AMBER_OF_RED_LINE, Band, amber_start, band, gear_text, segment_band, shift_cue};
use super::scale::{KMH_PER_MPS, MPH_PER_MPS, fraction, per_mps, top};
use super::speed::{digits, unit};
use crate::hud::{HudState, HudTelemetry, PursuitHud, TireCondition, VehicleCondition};
use crate::settings::hud::HudSettings;
use crate::settings::{SpeedUnits, Transmission};

const SETTLE: f32 = Motion::FULL.speed_spring;

/// A car at `rpm` with an 8000 rpm engine, a 7000 rpm red line and the gearbox's shift point `shift_up`.
fn revving(rpm: f32, shift_up: Option<f32>) -> HudState {
    HudState {
        rpm,
        max_rpm: 8000.0,
        red_line: 7000.0,
        telemetry: HudTelemetry { shift_up_rpm: shift_up, ..HudTelemetry::default() },
        ..HudState::default()
    }
}

/// The feed of `hud` at `elapsed` seconds on the frame clock, one 60 Hz frame long.
fn frame(hud: &HudState, elapsed: f32, settings: HudSettings) -> CustomFeed {
    CustomFeed {
        hud: Arc::new(hud.clone()),
        settings,
        elapsed_s: elapsed,
        delta_s: 1.0 / 60.0,
        ..CustomFeed::default()
    }
}

#[test]
fn the_spring_starts_on_its_target_and_settles_a_step_within_two_percent_in_about_60_ms_without_overshoot() {
    let mut spring = Spring::default();
    assert_eq!(spring.value(), None);
    assert_eq!(spring.step(0.0, 1.0 / 60.0, SETTLE), 0.0, "the first step starts on the target");
    // A step from 0 to 100, in 5 ms frames.
    let mut previous = 0.0;
    for frame in 1..=13 {
        let value = spring.step(100.0, 0.005, SETTLE);
        assert!(value <= 100.0, "never past the target: {value} at frame {frame}");
        assert!(value >= previous, "never back: {value} after {previous}");
        previous = value;
        if frame == 6 {
            assert!(value < 98.0, "30 ms in, still on its way: {value}");
        }
    }
    assert!(previous >= 98.0, "65 ms in, within 2%: {previous}");
    let settled = (0..100).map(|_| spring.step(100.0, 0.005, SETTLE)).last().expect("steps");
    assert!((settled - 100.0).abs() < 1e-3, "it comes to rest on the target: {settled}");
}

#[test]
fn the_spring_does_not_depend_on_the_frame_rate() {
    let (mut coarse, mut fine) = (Spring::default(), Spring::default());
    coarse.step(0.0, 0.0, SETTLE);
    fine.step(0.0, 0.0, SETTLE);
    let one = coarse.step(50.0, 0.03, SETTLE);
    let many = (0..6).map(|_| fine.step(50.0, 0.005, SETTLE)).last().expect("steps");
    assert!((one - many).abs() < 1e-3, "one 30 ms step {one} against six 5 ms steps {many}");
}

#[test]
fn the_spring_holds_without_time_and_lands_after_a_long_pause() {
    let mut spring = Spring::default();
    spring.step(10.0, 0.0, SETTLE);
    assert_eq!(spring.step(90.0, 0.0, SETTLE), 10.0, "no time, no move");
    let landed = spring.step(90.0, 5.0, SETTLE);
    assert!(landed <= 90.0 && (landed - 90.0).abs() < 1e-3, "a long frame lands on the target: {landed}");
    assert_eq!(spring.step(40.0, 0.016, 0.0), 40.0, "no settling time snaps to the target");
}

#[test]
fn a_pulse_fires_once_per_rising_edge() {
    let motion = Motion::FULL;
    let mut pulse = Pulse::default();
    assert!(!pulse.follow(false, 0.0));
    assert_eq!(pulse.level(0.0, &motion), 0.0, "no pulse before any edge");
    assert!(pulse.follow(true, 1.0), "the rising edge starts it");
    assert_eq!(pulse.level(1.0, &motion), 1.0);
    let midway = pulse.level(1.2, &motion);
    assert!(midway > 0.0 && midway < 1.0, "it fades over 400 ms: {midway}");
    for frame in 1..120 {
        assert!(!pulse.follow(true, 1.0 + frame as f32 / 60.0), "no repeat while the condition holds");
    }
    assert_eq!(pulse.level(2.0, &motion), 0.0, "over after its 400 ms");
    assert!(!pulse.follow(false, 3.0));
    assert!(pulse.follow(true, 3.5), "a new edge once the condition has been off");
    assert_eq!(pulse.level(3.5, &motion), 1.0);
}

#[test]
fn under_motion_reduction_the_pulse_is_a_static_highlight_as_long() {
    let motion = Motion::REDUCED;
    let mut pulse = Pulse::default();
    pulse.follow(true, 1.0);
    assert_eq!(pulse.level(1.0, &motion), 1.0);
    assert_eq!(pulse.level(1.0 + motion.pulse * 0.9, &motion), 1.0, "held, not fading");
    assert_eq!(pulse.level(1.01 + motion.pulse, &motion), 0.0, "for as long as the pulse");
}

#[test]
fn the_nitrous_pulse_follows_the_burn_and_a_car_without_nitrous_has_none() {
    let settings = HudSettings::default();
    let burning = HudState {
        has_nos: true,
        nos: 1.0,
        telemetry: HudTelemetry { nos_burning: true, ..HudTelemetry::default() },
        ..HudState::default()
    };
    let idle = HudState { telemetry: HudTelemetry::default(), ..burning.clone() };
    let mut memory = Memory::default();
    assert_eq!(memory.step(&frame(&idle, 0.0, settings), false).nos_pulse, 0.0);
    assert_eq!(memory.step(&frame(&burning, 0.1, settings), false).nos_pulse, 1.0, "the burn starts: the pulse");
    assert!(memory.step(&frame(&burning, 0.3, settings), false).nos_pulse > 0.0);
    assert_eq!(memory.step(&frame(&burning, 1.0, settings), false).nos_pulse, 0.0, "the burn goes on: no repeat");
    assert_eq!(memory.step(&frame(&burning, 2.0, settings), false).nos_pulse, 0.0);
    memory.step(&frame(&idle, 2.5, settings), false);
    assert_eq!(memory.step(&frame(&burning, 3.0, settings), false).nos_pulse, 1.0, "a new burn, a new pulse");

    let mut without = Memory::default();
    let no_part = HudState { has_nos: false, ..burning };
    assert_eq!(without.step(&frame(&no_part, 0.0, settings), false).nos_pulse, 0.0, "no nitrous, no pulse");
}

#[test]
fn the_memory_steps_once_per_frame() {
    let settings = HudSettings::default();
    let stopped = HudState::default();
    let moving = HudState { speed: 40.0, ..HudState::default() };
    let mut once = Memory::default();
    let mut twice = Memory::default();
    once.step(&frame(&stopped, 0.0, settings), false);
    twice.step(&frame(&stopped, 0.0, settings), false);
    let a = once.step(&frame(&moving, 0.02, settings), false);
    twice.step(&frame(&moving, 0.02, settings), false);
    let b = twice.step(&frame(&moving, 0.02, settings), false);
    assert_eq!(a, b, "a second layout of the same frame does not run the spring again");
    assert!(a.speed > 0.0 && a.speed < 40.0, "the scale is on its way: {}", a.speed);
}

#[test]
fn the_speedbreaker_hides_after_resting_full_in_free_roam_only() {
    let settings = HudSettings::default();
    let full = HudState { speedbreaker: 1.0, ..HudState::default() };
    let mut memory = Memory::default();
    assert!(!memory.step(&frame(&full, 10.0, settings), false).speedbreaker_hidden);
    assert!(!memory.step(&frame(&full, 10.0 + SPEEDBREAKER_HIDE_S - 0.1, settings), false).speedbreaker_hidden);
    assert!(memory.step(&frame(&full, 10.0 + SPEEDBREAKER_HIDE_S, settings), false).speedbreaker_hidden, "rested");
    let used = HudState { speedbreaker_active: true, ..full.clone() };
    assert!(!memory.step(&frame(&used, 14.0, settings), false).speedbreaker_hidden, "back as soon as it is used");

    let chased = HudState { pursuit: Some(PursuitHud::default()), ..full };
    let mut pursuit = Memory::default();
    pursuit.step(&frame(&chased, 0.0, settings), false);
    assert!(!pursuit.step(&frame(&chased, 60.0, settings), false).speedbreaker_hidden, "it stays in a pursuit");
}

#[test]
fn the_rev_bands_start_at_the_shift_point_and_turn_red_past_the_red_line() {
    let hud = revving(0.0, Some(6000.0));
    assert_eq!(amber_start(&hud), 6000.0);
    assert_eq!(band(5990.0, 6000.0, 7000.0), Band::Normal, "just below the amber start");
    assert_eq!(band(6500.0, 6000.0, 7000.0), Band::Amber, "inside the amber band");
    assert_eq!(band(7100.0, 6000.0, 7000.0), Band::Red, "past the red line");
    // 20 segments of 400 rpm: the one ending at 6000 is the last normal one.
    assert_eq!(segment_band(14, &hud), Band::Normal);
    assert_eq!(segment_band(15, &hud), Band::Amber);
    assert_eq!(segment_band(16, &hud), Band::Amber);
    assert_eq!(segment_band(17, &hud), Band::Red);
    assert_eq!(segment_band(19, &hud), Band::Red);
}

#[test]
fn without_a_shift_point_the_amber_band_starts_short_of_the_red_line() {
    let hud = revving(0.0, None);
    assert_eq!(amber_start(&hud), AMBER_OF_RED_LINE * 7000.0);
    assert_eq!(band(5900.0, amber_start(&hud), 7000.0), Band::Normal);
    assert_eq!(band(6000.0, amber_start(&hud), 7000.0), Band::Amber);
    assert!(Band::Amber.height() > Band::Normal.height(), "the bands read by shape too");
    assert_eq!(Band::Red.height(), Band::Amber.height());
}

#[test]
fn the_shift_cue_shows_for_the_automatic_box_wish_and_for_manual_past_the_shift_point() {
    let wish = HudState { shift_light: true, ..revving(5000.0, Some(6000.0)) };
    assert!(shift_cue(&wish, Transmission::Automatic), "the box wants the next gear");
    assert!(shift_cue(&wish, Transmission::Manual));
    let high = revving(6100.0, Some(6000.0));
    assert!(!shift_cue(&high, Transmission::Automatic), "the automatic box shifts by itself");
    assert!(shift_cue(&high, Transmission::Manual), "manual, past the shift point");
    assert!(shift_cue(&revving(6000.0, Some(6000.0)), Transmission::Manual), "manual, at the shift point");
    assert!(!shift_cue(&revving(5900.0, Some(6000.0)), Transmission::Manual), "manual, below it");
    assert!(!shift_cue(&revving(7900.0, None), Transmission::Manual), "the top gear has no next gear");
}

#[test]
fn the_gear_reads_r_n_or_its_number() {
    assert_eq!(gear_text(-1), "R");
    assert_eq!(gear_text(0), "N");
    assert_eq!(gear_text(4), "4");
}

#[test]
fn the_digits_and_the_scale_follow_the_unit() {
    // 83.2 m/s is 299.52 km/h and 186.1 mph: the digits are floored, as `display_speed` gives them.
    let metric = HudState { speed: 83.2, use_mph: false, ..HudState::default() };
    let imperial = HudState { use_mph: true, ..metric.clone() };
    assert_eq!(digits(&metric), "299");
    assert_eq!(digits(&imperial), "186");
    assert_eq!(digits(&metric), metric.display_speed().to_string());
    assert_eq!(unit(SpeedUnits::Metric), "KM/H");
    assert_eq!(unit(SpeedUnits::Imperial), "MPH");
    assert_eq!((per_mps(SpeedUnits::Metric), per_mps(SpeedUnits::Imperial)), (KMH_PER_MPS, MPH_PER_MPS));
    assert_eq!((top(SpeedUnits::Metric), top(SpeedUnits::Imperial)), (360.0, 225.0));
    assert!((fraction(50.0, SpeedUnits::Metric) - 0.5).abs() < 1e-6, "180 km/h is half of 360");
    let mph = fraction(50.0, SpeedUnits::Imperial);
    assert!((mph - 50.0 * MPH_PER_MPS / 225.0).abs() < 1e-6, "111.8 mph of 225: {mph}");
    assert_eq!(fraction(150.0, SpeedUnits::Metric), 1.0, "past the top, all lit");
    assert_eq!(fraction(-3.0, SpeedUnits::Metric), 0.0, "reversing reads as stopped");
}

#[test]
fn the_boost_bar_fills_to_thirty_psi_and_vacuum_is_empty() {
    assert_eq!(boost_fill(15.0), 0.5);
    assert_eq!(boost_fill(-4.0), 0.0);
    assert_eq!(boost_fill(45.0), 1.0);
}

#[test]
fn the_advanced_row_shows_only_the_values_the_game_has() {
    let base = HudState {
        rpm: 4213.0,
        telemetry: HudTelemetry { throttle: 0.78, lateral_g: -0.94, ..HudTelemetry::default() },
        ..HudState::default()
    };
    let texts = |fields: &[Field]| -> Vec<String> {
        fields
            .iter()
            .filter_map(|field| match field {
                Field::Text(text, _) => Some(text.clone()),
                Field::Tyres(_) => None,
            })
            .collect()
    };
    let plain = fields(&base);
    assert_eq!(texts(&plain), ["4210 rpm", "78%", "0.9 g"], "no boost without a turbo");
    assert!(!plain.iter().any(|field| matches!(field, Field::Tyres(_))), "no tyres without a known condition");

    let tires = [TireCondition::Sound, TireCondition::Punctured, TireCondition::Blown, TireCondition::Sound];
    let full = HudState {
        has_turbo: true,
        boost_psi: 12.44,
        condition: Some(VehicleCondition { tires, ..VehicleCondition::default() }),
        ..base
    };
    let all = fields(&full);
    assert_eq!(texts(&all), ["4210 rpm", "12.4 psi", "78%", "0.9 g"]);
    assert_eq!(all.last(), Some(&Field::Tyres(tires)), "the tyres close the row");
    assert_eq!(rpm_text(0.0), "0 rpm");
    assert_eq!((load_percent(1.4), load_percent(-0.2), load_percent(f32::NAN)), (100, 0, 0));
}
