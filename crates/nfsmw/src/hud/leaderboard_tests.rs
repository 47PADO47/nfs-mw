use blackbox_feng::package::{ObjectData, ObjectDef, ObjectKind, Package, Script, StringDef};
use blackbox_feng::{PackageId, Runtime};

use super::bind::{HudBinding, SCRIPT_INIT};
use super::leaders::{Entry, LeaderRow, Leaders};
use super::state::HudState;

const LEAVE: u32 = 0x0331_13AC;
const ENTER: u32 = 0x5079_C8F8;
const HIGHLIGHT: u32 = 0x249D_B7B7;
const GROUP: u32 = 0x5ED6_09A1;

fn object(kind: ObjectKind, guid: u32, name_hash: u32, parent: Option<u32>, scripts: &[(u32, i32)]) -> ObjectDef {
    let mut words = vec![0u32; 17];
    words[..4].fill(255);
    words[13] = 1.0f32.to_bits();
    ObjectDef {
        kind,
        guid,
        name_hash,
        flags: 0,
        resource: None,
        parent,
        data: ObjectData { words },
        string: (kind == ObjectKind::String).then(|| StringDef { text: "0".into(), ..Default::default() }),
        multi: None,
        scripts: scripts.iter().map(|(id, length)| Script { id: *id, length: *length, ..Default::default() }).collect(),
        responses: Vec::new(),
    }
}

/// The leaderboard group with its four rows, as `HUD_SingleRace.fng` has it (scripts and lengths as authored).
fn package() -> Package {
    let strings = [(SCRIPT_INIT, 0), (ENTER, 160), (LEAVE, 160)];
    let backing = [(SCRIPT_INIT, 0), (HIGHLIGHT, 1200)];
    let mut objects = vec![object(ObjectKind::Group, 1, GROUP, None, &[(SCRIPT_INIT, 0)])];
    for k in 0..4u32 {
        objects.push(object(ObjectKind::Image, 10 + k, 0x4FDD_D64C + k, Some(1), &backing));
        objects.push(object(ObjectKind::String, 20 + k, 0x2AF8_9FD7 + k, Some(1), &strings));
        objects.push(object(ObjectKind::String, 30 + k, 0x4183_E0A1 + k, Some(1), &strings));
        objects.push(object(ObjectKind::Image, 40 + k, 0x51F5_B673 + k, Some(1), &[(SCRIPT_INIT, 0)]));
    }
    Package {
        name: "HUD.fng".into(),
        file_name: String::new(),
        version: 0x20000,
        resources: Vec::new(),
        objects,
        button_count: 0,
        responses: Vec::new(),
        targets: Vec::new(),
    }
}

fn entry(place: u32, player: bool, name: &str, percent: f32) -> Entry {
    Entry { place, player, row: LeaderRow { name: name.into(), percent, points: 0.0, out: false, busted: false } }
}

fn leaders(racers: usize) -> Leaders {
    let names = ["stock", "ABEL", "CARLA", "DOM", "EVE", "FRED"];
    let entries = (0..racers).map(|i| entry(i as u32 + 1, i == 0, names[i], 50.0 - 5.0 * i as f32)).collect();
    Leaders::pick(entries, 3000.0, false)
}

fn text(rt: &Runtime, id: PackageId, guid: u32) -> String {
    let tree = rt.tree(id);
    tree.nodes.iter().find(|n| n.guid == guid).and_then(|n| n.text.clone()).unwrap_or_default()
}

fn visible(rt: &Runtime, id: PackageId, guid: u32) -> bool {
    rt.tree(id).nodes.iter().find(|n| n.guid == guid).unwrap().visible
}

/// Runs the HUD for `secs` seconds at 60 Hz.
fn run(binding: &mut HudBinding, rt: &mut Runtime, state: &HudState, secs: f32) {
    for _ in 0..(secs * 60.0) as usize {
        binding.advance_mirror_fade(1.0 / 60.0);
        binding.apply(rt, state);
        rt.update(1.0 / 60.0);
    }
}

fn setup() -> (Runtime, PackageId, HudBinding) {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let binding = HudBinding::new(&mut rt, id);
    (rt, id, binding)
}

#[test]
fn the_board_opens_with_the_names_and_the_places_and_highlights_the_player() {
    let (mut rt, id, mut binding) = setup();
    let state = HudState { leaders: Some(leaders(4)), ..Default::default() };
    run(&mut binding, &mut rt, &state, 0.2);
    assert_eq!(text(&rt, id, 20), "stock");
    assert_eq!(text(&rt, id, 23), "DOM");
    assert_eq!(text(&rt, id, 30), "1");
    assert_eq!(text(&rt, id, 33), "4");
    let backing = |guid| rt.script_of(rt.find_guid(id, guid).unwrap());
    assert_eq!(backing(10), Some(HIGHLIGHT), "the player's row is highlighted");
    assert_eq!(backing(11), Some(SCRIPT_INIT));
    assert!(visible(&rt, id, 1) && visible(&rt, id, 20) && visible(&rt, id, 33));
    assert!(!visible(&rt, id, 40), "the online headsets stay hidden");
}

#[test]
fn the_rows_slide_out_and_back_in_with_the_gaps() {
    let (mut rt, id, mut binding) = setup();
    let state = HudState { leaders: Some(leaders(4)), use_mph: true, ..Default::default() };
    // 90 updates later the rows leave; the text is still the names.
    run(&mut binding, &mut rt, &state, 1.52);
    let row = rt.find_guid(id, 21).unwrap();
    assert_eq!(rt.script_of(row), Some(LEAVE));
    assert_eq!(text(&rt, id, 21), "ABEL");
    // Once the leave has played (160 ms) they enter with the gaps: 5 percent of 3000 m is 150 m, 492 ft.
    run(&mut binding, &mut rt, &state, 0.25);
    assert_eq!(rt.script_of(row), Some(ENTER));
    assert_eq!(text(&rt, id, 20), "-----");
    assert_eq!(text(&rt, id, 21), "-492 FT");
    assert_eq!(text(&rt, id, 23), "-1,476 FT");
    assert_eq!(text(&rt, id, 30), "1", "the place keeps its number");
    // A period later the names come back.
    run(&mut binding, &mut rt, &state, 1.9);
    assert_eq!(text(&rt, id, 21), "ABEL");
}

#[test]
fn the_metric_setting_prints_metres() {
    let (mut rt, id, mut binding) = setup();
    let state = HudState { leaders: Some(leaders(4)), use_mph: false, ..Default::default() };
    run(&mut binding, &mut rt, &state, 1.9);
    assert_eq!(text(&rt, id, 21), "-150 M");
    // The imperial setting reads feet, a swap later.
    let imperial = HudState { use_mph: true, ..state };
    run(&mut binding, &mut rt, &imperial, 0.1);
    assert_eq!(text(&rt, id, 21), "-492 FT");
}

#[test]
fn a_small_race_shows_only_its_racers_and_a_single_racer_none() {
    let (mut rt, id, mut binding) = setup();
    let state = HudState { leaders: Some(leaders(2)), ..Default::default() };
    run(&mut binding, &mut rt, &state, 0.2);
    assert!(visible(&rt, id, 20) && visible(&rt, id, 21));
    assert!(!visible(&rt, id, 22) && !visible(&rt, id, 23), "no row for a third racer");
    assert!(!visible(&rt, id, 12), "nor its backing");
    let alone = HudState { leaders: Some(leaders(1)), ..Default::default() };
    run(&mut binding, &mut rt, &alone, 0.1);
    assert!(!visible(&rt, id, 1), "a race of one has no board");
}

#[test]
fn the_board_goes_with_the_race_and_starts_again_with_the_next() {
    let (mut rt, id, mut binding) = setup();
    let state = HudState { leaders: Some(leaders(4)), ..Default::default() };
    run(&mut binding, &mut rt, &state, 1.7);
    run(&mut binding, &mut rt, &HudState::default(), 0.1);
    assert!(!visible(&rt, id, 1), "no race, no board");
    run(&mut binding, &mut rt, &state, 0.1);
    assert_eq!(text(&rt, id, 21), "ABEL", "the next race opens with the names again");
    assert_eq!(rt.script_of(rt.find_guid(id, 21).unwrap()), Some(SCRIPT_INIT));
}
