use super::*;

fn zone(compare: f32, value: f32) -> Option<ZoneSample> {
    Some(ZoneSample { compare, value })
}

#[test]
fn a_low_chance_forces_dry_and_a_high_chance_clears_the_override() {
    let mut w = Weather::new(0.2);
    w.set_chance_of_rain(0.005);
    assert_eq!(w.override_(), Some(Override::ForcedDry));
    w.set_chance_of_rain(0.5);
    assert_eq!(w.override_(), None);
}

#[test]
fn the_chance_is_clamped_before_the_threshold() {
    let mut w = Weather::new(0.2);
    w.set_chance_of_rain(-1.0);
    assert_eq!(w.override_(), Some(Override::ForcedDry));
    w.set_chance_of_rain(2.0);
    assert_eq!(w.override_(), None);
}

#[test]
fn a_forced_dry_override_reads_as_no_rain_when_ungated() {
    let mut w = Weather::new(0.2);
    w.set_chance_of_rain(0.0);
    assert_eq!(w.sample_rain(0.0, false, zone(1.0, 0.8)), 0.0);
}

#[test]
fn the_gate_ignores_the_override_and_squares_the_roll() {
    let mut w = Weather::new(0.2);
    w.set_chance_of_rain(0.0);
    // Gated: the override is ignored and the roll 0.5 squares to 0.25, which passes a compare of 0.3.
    assert_eq!(w.sample_rain(0.5, true, zone(0.3, 0.8)), 0.8);
    // Ungated with no override: the raw roll 0.5 fails the same compare.
    let fresh = Weather::new(0.2);
    assert_eq!(fresh.sample_rain(0.5, false, zone(0.3, 0.8)), 0.0);
}

#[test]
fn the_regional_gate_varies_with_position_and_time() {
    assert!(regional_gate(1.0, 0.0) < 0.001);
    assert!(regional_gate(1.0, 9000.0) > 0.99);
    assert!(regional_gate(252.0, 0.0) > 0.99);
}

#[test]
fn a_missing_zone_entry_gives_no_rain() {
    let w = Weather::new(0.2);
    assert_eq!(w.sample_rain(0.0, false, None), 0.0);
}

#[test]
fn rain_eases_toward_its_target_and_stays_within_one() {
    let mut w = Weather::new(0.2);
    for _ in 0..200 {
        w.step(1.0 / 60.0, 0.9, 0.0);
    }
    assert!(w.rain > 0.85 && w.rain <= 1.0, "rain = {}", w.rain);
}

#[test]
fn rain_clamps_at_one_when_the_target_is_above_it() {
    let mut w = Weather::new(0.2);
    for _ in 0..600 {
        w.step(0.5, 5.0, 0.0);
    }
    assert!(w.rain <= 1.0);
}

#[test]
fn a_faint_rain_snaps_to_zero_when_its_target_is_zero() {
    let mut w = Weather::new(0.2);
    w.rain = 0.005;
    w.step(0.1, 0.0, 0.0);
    assert_eq!(w.rain, 0.0);
}

#[test]
fn wind_settles_on_its_floor_when_its_target_is_zero() {
    let mut w = Weather::new(0.2);
    w.wind = 0.205;
    w.step(0.1, 0.0, 0.0);
    assert_eq!(w.wind, 0.2);
}

#[test]
fn the_heading_drifts_and_wraps_past_360() {
    let mut w = Weather::new(0.2);
    w.step(1.0, 0.0, 0.0);
    assert!((w.heading - HEADING_RATE).abs() < 1e-4);
    w.heading = 359.0;
    w.step(1.0, 0.0, 0.0);
    assert_eq!(w.heading, 0.0);
}

#[test]
fn a_zero_frame_time_uses_the_fallback() {
    let mut w = Weather::new(0.2);
    w.step(0.0, 1.0, 0.0);
    assert!((w.heading - HEADING_RATE * FALLBACK_DT).abs() < 1e-6);
}
