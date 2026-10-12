use super::drops::{DropPool, POOL_SIZE, Tuning};
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
fn the_base_roll_is_effectively_zero_over_the_whole_range() {
    for seconds in [0.0_f32, 1.0, 179.0, 359.0, 360.0, 499.0, 500.0] {
        assert!(base_roll(seconds) < 1e-4, "base_roll({seconds}) = {}", base_roll(seconds));
    }
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

fn tuning() -> Tuning {
    Tuning {
        fall_speed: 10.0,
        box_half: [5.0, 5.0, 5.0],
        ground_low: 0.0,
        ground_high: 50.0,
        wind_scale: 1.0,
        spawn_rate: 1000.0,
    }
}

fn counting_rand() -> impl FnMut() -> f32 {
    let mut n = 0.0_f32;
    move || {
        n = (n + 0.37) % 2.0 - 1.0;
        n
    }
}

#[test]
fn the_pool_has_the_spec_size() {
    assert_eq!(DropPool::new().drops().len(), POOL_SIZE);
}

#[test]
fn no_drops_spawn_while_it_is_dry() {
    let mut pool = DropPool::new();
    pool.update(0.1, [0.0; 3], [0.0; 2], 0.0, &tuning(), &mut counting_rand());
    assert_eq!(pool.falling(), 0);
}

#[test]
fn drops_spawn_in_the_box_while_it_rains() {
    let mut pool = DropPool::new();
    pool.update(0.1, [0.0, 20.0, 0.0], [0.0; 2], 1.0, &tuning(), &mut counting_rand());
    assert!(pool.falling() > 0);
    for d in pool.drops().iter().filter(|d| d.falling) {
        assert!((d.pos[0]).abs() <= 5.0 && (d.pos[2]).abs() <= 5.0);
        assert!((d.pos[1] - 20.0).abs() <= 5.0);
    }
}

#[test]
fn falling_drops_fall_and_are_recycled_at_the_ground() {
    let mut pool = DropPool::new();
    pool.update(0.1, [0.0, 2.0, 0.0], [0.0; 2], 1.0, &tuning(), &mut counting_rand());
    let before = pool.falling();
    assert!(before > 0);
    // Long enough for every drop to pass the ground at 0 m (fall speed 10 m/s), with no rain.
    for _ in 0..20 {
        pool.update(0.5, [0.0; 3], [0.0; 2], 0.0, &tuning(), &mut counting_rand());
    }
    assert_eq!(pool.falling(), 0);
}

#[test]
fn the_wind_moves_a_falling_drop_sideways() {
    let mut pool = DropPool::new();
    pool.update(0.1, [0.0, 20.0, 0.0], [0.0; 2], 1.0, &tuning(), &mut counting_rand());
    let x0 = pool.drops().iter().find(|d| d.falling).unwrap().pos[0];
    pool.update(0.1, [0.0, 20.0, 0.0], [2.0, 0.0], 0.0, &tuning(), &mut counting_rand());
    let x1 = pool.drops().iter().find(|d| d.falling).unwrap().pos[0];
    assert!((x1 - x0 - 0.2).abs() < 1e-4, "moved {}", x1 - x0);
}
