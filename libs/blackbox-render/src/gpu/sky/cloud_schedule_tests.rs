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
    assert_eq!(schedule.plan(key(), false, true), Some(Rebuild::Full));
    assert_eq!(schedule.plan(key(), false, true), None, "an unchanged key holds the map");
    assert_eq!(schedule.plan(key(), true, true), Some(Rebuild::Full), "a camera move rebuilds a held map at once");
    assert_eq!(schedule.plan(Key { coverage: 1.0, ..key() }, false, true), Some(Rebuild::Full));
    assert_eq!(
        schedule.plan(Key { coverage: 1.0, sun: [1.0, 0.0, 0.0, 0.0], ..key() }, false, true),
        Some(Rebuild::Full)
    );
    assert_eq!(
        schedule.plan(Key { coverage: 1.0, ..key() }, false, true),
        Some(Rebuild::Full),
        "sun back is a big move"
    );
    let jumped = Key { coverage: 1.0, scroll: 900.0, ..key() };
    assert_eq!(schedule.plan(jumped, false, true), Some(Rebuild::Full), "a clock jump moves every cloud at once");
}

#[test]
fn wind_rebuilds_one_band_per_call_and_every_band_in_turn() {
    let mut schedule = Schedule::default();
    schedule.plan(key(), false, true);
    let mut seen = Vec::new();
    let wind = |frame: u32| Key { scroll: 0.001 * frame as f32, ..key() };
    for frame in 1..=BANDS {
        let Some(Rebuild::Band(band)) = schedule.plan(wind(frame), false, true) else { panic!("wind rebuilds a band") };
        seen.push(band);
    }
    seen.sort_unstable();
    assert_eq!(seen, (0..BANDS).collect::<Vec<_>>(), "continuous wind refreshes each band once a turn");
    let finishing = (0..BANDS).filter(|_| schedule.plan(wind(BANDS), false, true).is_some()).count();
    assert_eq!(finishing, BANDS as usize - 1, "once wind stops the bands still stale catch up");
    assert_eq!(schedule.plan(wind(BANDS), false, true), None, "then the map is current");
}

#[test]
fn a_wind_change_that_stops_finishes_the_remaining_bands() {
    let mut schedule = Schedule::default();
    schedule.plan(key(), false, true);
    let moved = Key { scroll: 0.5, ..key() };
    let bands: Vec<_> = (0..BANDS + 2).map(|_| schedule.plan(moved, false, true)).collect();
    let rebuilt = bands.iter().flatten().count();
    assert_eq!(rebuilt, BANDS as usize);
    assert!(bands[BANDS as usize..].iter().all(Option::is_none));
}

#[test]
fn bands_tile_the_map_rows_exactly() {
    let covered: u32 = (0..BANDS).map(|band| Rebuild::Band(band).rows(384).1).sum();
    assert_eq!(covered, 384);
    assert_eq!(Rebuild::Band(0).rows(384), (0, 20), "the horizon rows march the most samples: a short band");
    assert_eq!(Rebuild::Band(BANDS - 1).rows(384), (324, 60));
    assert!((0..BANDS).all(|band| Rebuild::Band(band).rows(384).1 > 0));
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
    schedule.plan(still(10.0), false, true);
    let rebuilt = (1..=120).filter(|&i| schedule.plan(still(10.0 + i as f32 * frame), false, true).is_some()).count();
    let turns = rebuilt / BANDS as usize;
    assert!(turns <= 3, "a moving sun must let the map settle between steps: {rebuilt} passes in 120 frames");
    assert_eq!(at(0.0), at(2.0), "below the unlit height the sun no longer changes the key");
    assert_eq!(at(0.0).1, [0.0; 3], "the scene light (the moon at night) never reaches the march");
    assert_ne!(at(0.0), at(12.0));
    assert_eq!(sun_key(Some([f32::NAN; 3]), scene_sun, scene_light), at(0.0), "invalid input is the unlit key");
}

#[test]
fn the_other_views_of_a_frame_do_not_march_bands() {
    let mut schedule = Schedule::default();
    assert_eq!(schedule.plan(key(), false, false), Some(Rebuild::Full), "the first view of all builds the map");
    let wind = |frame: u32| Key { scroll: 0.001 * frame as f32, ..key() };
    let mut seen = Vec::new();
    for frame in 1..=BANDS {
        // Two reflection faces and a road capture come before the frame's own view, and a mirror after it.
        assert_eq!((0..3).filter_map(|_| schedule.plan(wind(frame), false, false)).count(), 0);
        let Some(Rebuild::Band(band)) = schedule.plan(wind(frame), false, true) else { panic!("one band a frame") };
        seen.push(band);
        assert_eq!(schedule.plan(wind(frame), false, false), None);
    }
    seen.sort_unstable();
    assert_eq!(seen, (0..BANDS).collect::<Vec<_>>());
    let big = Key { coverage: 1.0, ..wind(BANDS) };
    assert_eq!(schedule.plan(big, false, false), Some(Rebuild::Full), "a visible jump rebuilds from any view");
}

#[test]
fn a_camera_move_while_bands_are_in_turn_needs_no_full_march() {
    let mut schedule = Schedule::default();
    schedule.plan(key(), false, true);
    let wind = |frame: u32| Key { scroll: 0.001 * frame as f32, ..key() };
    for frame in 1..=8 {
        let moved = frame % 2 == 0;
        assert!(matches!(schedule.plan(wind(frame), moved, true), Some(Rebuild::Band(_))), "frame {frame}");
    }
    // Wind stops: the stale bands finish, then a move is the only reason left to march.
    while schedule.plan(wind(8), false, true).is_some() {}
    assert_eq!(schedule.plan(wind(8), true, true), Some(Rebuild::Full));
}

#[test]
fn new_panorama_pictures_or_a_big_blend_move_rebuild_every_row_and_a_small_one_a_band() {
    let mut schedule = Schedule::default();
    let on = |epoch: f32, blend: f32| Key { pano: [epoch, blend, 0.0, 0.0], ..key() };
    assert_eq!(schedule.plan(on(1.0, 0.0), false, true), Some(Rebuild::Full));
    assert_eq!(schedule.plan(on(2.0, 0.0), false, true), Some(Rebuild::Full), "other pictures");
    assert_eq!(schedule.plan(on(2.0, 0.5), false, true), Some(Rebuild::Full), "a jump of the blend");
    assert!(matches!(schedule.plan(on(2.0, 0.51), false, true), Some(Rebuild::Band(_))), "a drift of it");
}
