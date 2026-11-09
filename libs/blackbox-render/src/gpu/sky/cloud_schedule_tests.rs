use super::*;

fn key() -> Key {
    Key {
        scroll: 0.0,
        coverage: 0.5,
        sun: [0.0, 0.0, 1.0, 0.0],
        light: [0.0, 0.0, -1.0],
        warmth: 1.0,
        pano: [-1.0, 0.0, 0.0, 0.0],
    }
}

#[test]
fn first_plan_and_big_changes_rebuild_every_row() {
    let mut schedule = Schedule::default();
    assert_eq!(schedule.plan(key(), false), Some(Rebuild::Full));
    assert_eq!(schedule.plan(key(), false), None, "an unchanged key holds the map");
    assert_eq!(schedule.plan(key(), true), Some(Rebuild::Full), "a camera teleport rebuilds at once");
    assert_eq!(schedule.plan(Key { coverage: 1.0, ..key() }, false), Some(Rebuild::Full));
    assert_eq!(schedule.plan(Key { coverage: 1.0, sun: [1.0, 0.0, 0.0, 0.0], ..key() }, false), Some(Rebuild::Full));
    assert_eq!(schedule.plan(Key { coverage: 1.0, ..key() }, false), Some(Rebuild::Full), "sun back is a big move");
    let jumped = Key { coverage: 1.0, scroll: 900.0, ..key() };
    assert_eq!(schedule.plan(jumped, false), Some(Rebuild::Full), "a clock jump moves every cloud at once");
}

#[test]
fn wind_rebuilds_one_band_per_call_and_every_band_in_turn() {
    let mut schedule = Schedule::default();
    schedule.plan(key(), false);
    let mut seen = Vec::new();
    let wind = |frame: u32| Key { scroll: 0.001 * frame as f32, ..key() };
    for frame in 1..=4 {
        let Some(Rebuild::Band(band)) = schedule.plan(wind(frame), false) else { panic!("wind rebuilds a band") };
        seen.push(band);
    }
    seen.sort_unstable();
    assert_eq!(seen, [0, 1, 2, 3], "continuous wind refreshes each band every four calls");
    let finishing = (0..BANDS).filter(|_| schedule.plan(wind(4), false).is_some()).count();
    assert_eq!(finishing, BANDS as usize - 1, "once wind stops the bands still stale catch up");
    assert_eq!(schedule.plan(wind(4), false), None, "then the map is current");
}

#[test]
fn a_wind_change_that_stops_finishes_the_remaining_bands() {
    let mut schedule = Schedule::default();
    schedule.plan(key(), false);
    let moved = Key { scroll: 0.5, ..key() };
    let bands: Vec<_> = (0..6).map(|_| schedule.plan(moved, false)).collect();
    let rebuilt = bands.iter().flatten().count();
    assert_eq!(rebuilt, BANDS as usize);
    assert!(bands[BANDS as usize..].iter().all(Option::is_none));
}

#[test]
fn bands_tile_the_map_rows_exactly() {
    let covered: u32 = (0..BANDS).map(|band| Rebuild::Band(band).rows(384).1).sum();
    assert_eq!(covered, 384);
    assert_eq!(Rebuild::Band(0).rows(384), (0, 96));
    assert_eq!(Rebuild::Band(3).rows(384), (288, 96));
    assert_eq!(Rebuild::Full.rows(384), (0, 384));
    let rows: Vec<_> = (0..BANDS).map(|band| Rebuild::Band(band).rows(10)).collect();
    assert_eq!(rows.iter().map(|r| r.1).sum::<u32>(), 10);
    assert!(rows.windows(2).all(|pair| pair[0].0 + pair[0].1 == pair[1].0));
}

#[test]
fn a_celestial_sun_settles_in_steps_and_freezes_once_unlit() {
    let (scene_sun, scene_light) = ([0.1, 0.2, 0.9, 1.0], [0.3, 0.0, -0.9]);
    assert_eq!(sun_key(None, scene_sun, scene_light), (scene_sun, scene_light), "no cycle: the scene's own key");
    let at = |hour: f32| {
        let angle = (hour - 6.0) * std::f32::consts::PI / 12.0;
        sun_key(Some([angle.cos() * 0.94, 0.34, angle.sin() * 0.94]), scene_sun, scene_light)
    };
    // One 60 Hz frame of the default clock (150 s per game hour).
    let frame = 1.0 / 9000.0;
    let mut schedule = Schedule::default();
    let still = |hour: f32| Key { sun: at(hour).0, light: at(hour).1, ..key() };
    schedule.plan(still(10.0), false);
    let rebuilt = (1..=120).filter(|&i| schedule.plan(still(10.0 + i as f32 * frame), false).is_some()).count();
    assert!(rebuilt <= 12, "a moving sun must let the map settle between steps: {rebuilt} of 120 frames");
    assert_eq!(at(0.0), at(2.0), "below the unlit height the sun no longer changes the key");
    assert_eq!(at(0.0).1, [0.0; 3], "the scene light (the moon at night) never reaches the march");
    assert_ne!(at(0.0), at(12.0));
    assert_eq!(sun_key(Some([f32::NAN; 3]), scene_sun, scene_light), at(0.0), "invalid input is the unlit key");
}

#[test]
fn new_panorama_pictures_or_a_big_blend_move_rebuild_every_row_and_a_small_one_a_band() {
    let mut schedule = Schedule::default();
    let on = |epoch: f32, blend: f32| Key { pano: [epoch, blend, 0.0, 0.0], ..key() };
    assert_eq!(schedule.plan(on(1.0, 0.0), false), Some(Rebuild::Full));
    assert_eq!(schedule.plan(on(2.0, 0.0), false), Some(Rebuild::Full), "other pictures");
    assert_eq!(schedule.plan(on(2.0, 0.5), false), Some(Rebuild::Full), "a jump of the blend");
    assert!(matches!(schedule.plan(on(2.0, 0.51), false), Some(Rebuild::Band(_))), "a drift of it");
}
