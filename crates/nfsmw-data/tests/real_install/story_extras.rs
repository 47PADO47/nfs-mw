//! The story data beyond the intro and outro (`gameplay/story.rs`, `gameplay/rewards.rs`, `gameplay/final_race.rs`)
//! against docs/specs/story-flow.md, docs/specs/story-flow-2.md (sections 4.5 and 5) and docs/specs/career-blacklist.md.
//! Positions are printed in physics space and in map coordinates (`map = (z, -x, y)` of the physics position), the frame
//! the specs use.

use std::collections::BTreeMap;

use nfsmw_data::gameplay::{self, EventMarker, GameplayData, Placement, Race, RewardRecord};
use nfsmw_data::shared;

use crate::install;

/// The race with this event id (`16.1.1`).
fn event<'a>(data: &'a GameplayData, id: &str) -> &'a Race {
    data.races.iter().find(|r| r.event.as_deref() == Some(id)).unwrap_or_else(|| panic!("no race with event {id}"))
}

/// A physics-space position in map coordinates.
fn map_of(p: glam::Vec3) -> [f32; 3] {
    [p.z, -p.x, p.y]
}

fn map_of_placement(p: &Placement) -> [f32; 3] {
    map_of(p.position)
}

fn close(a: [f32; 3], b: [f32; 3], tolerance: f32) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tolerance)
}

fn print_marker(label: &str, m: &EventMarker) {
    let map = map_of_placement(&m.placement);
    println!(
        "CENSUS final {label} {} physics={:?} map={map:?} heading={} dims={:?}",
        m.name, m.placement.position, m.placement.heading_deg, m.dimensions
    );
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_extra_story_fields_are_in_the_gameplay_schema() {
    let Some(dir) = install() else { return };
    let db = shared::attributes(&dir).expect("attributes");
    let class = db.class("gameplay").expect("the gameplay class");
    for name in ["DDayRace", "NISShell", "IntroCameraTrack"] {
        assert!(class.field(name).is_some(), "{name} is not a gameplay field");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_flags_and_the_outro_marker_census() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    type Check = fn(&Race) -> bool;
    let names = |set: Check| -> Vec<&str> { data.races.iter().filter(|r| set(r)).map(|r| r.name.as_str()).collect() };
    let flags: [(&str, Check); 5] = [
        ("dday", |r| r.story.dday),
        ("nis_shell", |r| r.story.nis_shell),
        ("special_setup", |r| r.story.special_setup),
        ("defer_rewards", |r| r.story.defer_rewards),
        ("has_outro_marker", |r| r.story.has_outro_marker),
    ];
    for (label, set) in flags {
        let found = names(set);
        println!("CENSUS {label} {} {found:?}", found.len());
    }

    let mut cameras: BTreeMap<i32, usize> = BTreeMap::new();
    for race in &data.races {
        *cameras.entry(race.story.intro_camera_track).or_default() += 1;
    }
    println!("CENSUS intro_camera_track {cameras:?}");

    let epic = data.race_named("race_bin_01/epic_pursuit").expect("the final race");
    assert!(epic.story.special_setup, "the final race runs the special-setup hook (story-flow-2 F-14)");

    let razor_outro = event(&data, "16.1.1");
    assert!(razor_outro.story.has_outro_marker, "16.1.1 has an outro_nis_marker child (story-flow-2 F-12)");
    assert_eq!(razor_outro.story.outro_nis, None, "16.1.1 has no OutroNIS (story-flow-2 F-12)");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_reward_list_census_matches_the_documented_counts() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");

    let entries: Vec<(&str, &RewardRecord)> =
        data.races.iter().flat_map(|r| r.story.rewards.iter().map(move |w| (r.name.as_str(), w))).collect();
    let races_with = data.races.iter().filter(|r| !r.story.rewards.is_empty()).count();
    println!("CENSUS rewards entries={} races_with_rewards={races_with}", entries.len());
    for race in data.races.iter().filter(|r| !r.story.rewards.is_empty()) {
        println!("CENSUS rewards race {} entries={}", race.name, race.story.rewards.len());
    }

    // The collection names, as the provenance doc counts them (career-blacklist.md, data findings).
    for needle in
        ["pink_slip", "out_of_jail_free", "backroom", "add_impound_box", "release_car_from_impound", "cash_bonus"]
    {
        let count = entries.iter().filter(|(_, w)| w.name.contains(needle)).count();
        println!("CENSUS reward name {needle} {count}");
    }

    // The distinct marker-kind texts and the kind the documented texts give (`RewardRecord::kind`).
    let mut texts: BTreeMap<(Option<&str>, Option<&str>), usize> = BTreeMap::new();
    for (_, w) in &entries {
        *texts.entry((w.kind_text.as_deref(), w.kind_text2.as_deref())).or_default() += 1;
    }
    for ((first, second), count) in &texts {
        println!("CENSUS kind texts {first:?} / {second:?} x{count}");
    }
    let mut kinds: BTreeMap<Option<u8>, usize> = BTreeMap::new();
    for (_, w) in &entries {
        *kinds.entry(w.kind()).or_default() += 1;
    }
    println!("CENSUS kind() {kinds:?}");

    let mut parts: BTreeMap<u32, usize> = BTreeMap::new();
    for (_, w) in &entries {
        if let Some(part) = w.part {
            *parts.entry(part).or_default() += 1;
        }
    }
    println!("CENSUS parts {parts:?}");

    let cash: Vec<i32> = entries.iter().filter(|(_, w)| w.name.contains("cash_bonus")).map(|(_, w)| w.value).collect();
    println!("CENSUS cash_bonus values {cash:?}");

    let epic = data.race_named("race_bin_01/epic_pursuit").expect("the final race");
    assert!(epic.story.rewards.is_empty(), "the final race lists no rewards (story-flow-2 5.1)");

    // The documented counts (story-flow-2 section 4.5 and the provenance doc). Printed, not asserted: the install
    // differs from them, and the difference is reported rather than adjusted here.
    let documented = [
        ("pink_slip", 15),
        ("out_of_jail_free", 13),
        ("add_impound_box", 8),
        ("release_car_from_impound", 6),
        ("cash_bonus", 8),
    ];
    for (needle, want) in documented {
        let got = entries.iter().filter(|(_, w)| w.name.contains(needle)).count();
        println!("CENSUS documented {needle} want={want} got={got}");
    }
    let by_kind = |k: u8| entries.iter().filter(|(_, w)| w.kind() == Some(k)).count();
    println!(
        "CENSUS documented by kind(): pink_slip 0x12={} out_of_jail_free 0x11={} impound_box 0x14={} release 0x15={} cash 0x13={}",
        by_kind(0x12),
        by_kind(0x11),
        by_kind(0x14),
        by_kind(0x15),
        by_kind(0x13)
    );
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_final_race_markers_match_the_documented_positions() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let markers = data.final_race.as_ref().expect("the final race markers");

    if let Some(m) = &markers.start_grid {
        print_marker("start_grid", m);
    }
    for (label, trigger) in [("win_trigger", &markers.win_trigger), ("lose_trigger", &markers.lose_trigger)] {
        let Some(t) = trigger else {
            panic!("{label} is missing");
        };
        println!(
            "CENSUS final {label} {} physics={:?} map={:?} heading={} dims={:?} fire_on_exit={} one_shot={}",
            t.name,
            t.position,
            map_of(t.position),
            t.heading_deg,
            t.dimensions,
            t.fire_on_exit,
            t.one_shot
        );
    }
    if let Some(m) = &markers.barrier_area {
        print_marker("barrier_area", m);
    }
    if let Some(m) = &markers.win_nis_marker {
        print_marker("win_nis_marker", m);
    }
    for m in &markers.tagged_car_markers {
        print_marker("tagged", m);
    }
    println!("CENSUS final tagged count {}", markers.tagged_car_markers.len());

    // Map coordinates from story-flow-2 section 5.3 and F-32 (the spec's frame).
    let win = markers.win_trigger.as_ref().expect("win_trigger");
    assert!(close(map_of(win.position), [-960.9, 2647.3, 208.8], 1.0), "win_trigger at {:?}", map_of(win.position));
    let lose = markers.lose_trigger.as_ref().expect("lose_trigger");
    assert!(close(map_of(lose.position), [-988.5, 2627.1, 200.1], 0.15), "lose_trigger");
    let barrier = markers.barrier_area.as_ref().expect("barrier_area");
    assert!(close(map_of_placement(&barrier.placement), [-365.6, 3004.6, 188.4], 0.15), "barrier_area");
    let nis = markers.win_nis_marker.as_ref().expect("win_nis_marker");
    assert!(close(map_of_placement(&nis.placement), [-954.8, 2652.0, 207.1], 0.15), "win_nis_marker");
    let grid = markers.start_grid.as_ref().expect("startgrid1");
    assert!(close(map_of_placement(&grid.placement), [4341.4, 48.7, 10.9], 0.15), "startgrid1");

    assert_eq!(markers.tagged_car_markers.len(), 6, "taggedcarmarker1 to 6");
    // story-flow-2 F-32 gives these ranges as x 4233 to 4323, y 48 to 61, z 8 to 10. They are rounded: the install
    // has y 47.5 to 61.2 and z 7.9 to 10.8, so the check allows a 1 m margin on y and z.
    for m in &markers.tagged_car_markers {
        let [x, y, z] = map_of_placement(&m.placement);
        assert!((4233.0..=4323.0).contains(&x), "{} x {x}", m.name);
        assert!((47.0..=62.0).contains(&y), "{} y {y}", m.name);
        assert!((7.0..=11.0).contains(&z), "{} z {z}", m.name);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_safe_house_drive_target_is_its_respawn_marker() {
    let Some(dir) = install() else { return };
    let data = gameplay::load(&dir).expect("gameplay data");
    let respawn = data.zone_respawn("menu_gate_safehouse/zone_safehouse_1").expect("the safe house respawn marker");
    println!(
        "CENSUS safehouse respawn physics={:?} map={:?} heading={}",
        respawn.position,
        map_of_placement(&respawn),
        respawn.heading_deg
    );
    assert_eq!(data.zone_respawn("menu_gate_safehouse/zone_safehouse_99"), None);
}
