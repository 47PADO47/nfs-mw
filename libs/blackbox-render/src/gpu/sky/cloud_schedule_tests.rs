use super::*;

fn key() -> Key {
    Key { scroll: 0.0, coverage: 0.5, sun: [0.0, 0.0, 1.0, 0.0], light: [0.0, 0.0, -1.0], warmth: 1.0 }
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
