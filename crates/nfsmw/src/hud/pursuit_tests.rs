//! The pursuit board and the cool-down bar, bound to a small package with the objects of the real one (names and
//! parents as in `HUD_SingleRace.fng`, see `docs/specs/hud-pursuit.md`).

use blackbox_feng::package::{ObjectData, ObjectDef, ObjectKind, Package, StringDef};
use blackbox_feng::{PackageId, Runtime, fe_hash_upper};
use glam::Vec3;

use super::bind::HudBinding;
use super::pursuit::PursuitHud;
use super::state::HudState;

/// The pursuit board's group, its timer, its bounty row, and the radar's group with the cool-down bar under it.
const BOARD: u32 = 1;
const TIMER: u32 = 2;
const BOUNTY_ROW: u32 = 3;
const RADAR: u32 = 4;
const RADAR_BACKING: u32 = 12;
const COOLDOWN: u32 = 5;
const BAR: u32 = 6;
const TRACK: u32 = 7;
const HIDING: u32 = 8;
const HIDING_ICON: u32 = 9;
const HEAT: u32 = 10;
const SPEEDOMETER: u32 = 11;

fn named(kind: ObjectKind, guid: u32, name_hash: u32, parent: Option<u32>, size: Vec3) -> ObjectDef {
    let mut words = vec![0u32; 17];
    for word in words.iter_mut().take(4) {
        *word = 255;
    }
    words[10 + 3] = 1.0f32.to_bits();
    words[14] = size.x.to_bits();
    words[15] = size.y.to_bits();
    words[16] = size.z.to_bits();
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
        scripts: Vec::new(),
        responses: Vec::new(),
    }
}

fn object(kind: ObjectKind, guid: u32, name: &str, parent: Option<u32>, size: Vec3) -> ObjectDef {
    named(kind, guid, fe_hash_upper(name), parent, size)
}

fn package() -> Package {
    // Groups have unit size in the package (a zero size collapses the world transform of everything under them).
    let none = Vec3::ONE;
    let objects = vec![
        // The board's group is the hash of `PursuitBoardGroup` (0xde89e070), a name the package holds as a hash only.
        named(ObjectKind::Group, BOARD, 0xDE89_E070, None, none),
        object(ObjectKind::String, TIMER, "PursuitTime", Some(BOARD), none),
        named(ObjectKind::Group, BOUNTY_ROW, 0x8674_E6D4, Some(BOARD), none),
        // The radar's group (`0xab9c1d4d`) holds its backing (the layout's radar object) and the cool-down bar.
        named(ObjectKind::Group, RADAR, 0xAB9C_1D4D, None, none),
        named(ObjectKind::Image, RADAR_BACKING, 0x839E_7D77, Some(RADAR), none),
        named(ObjectKind::Group, COOLDOWN, 0x84A2_26EC, Some(RADAR), none),
        named(ObjectKind::Image, BAR, 0xCF81_7638, Some(COOLDOWN), Vec3::new(216.0, 5.0, 0.0)),
        named(ObjectKind::Image, TRACK, 0xA1EB_7E6E, Some(COOLDOWN), Vec3::new(216.0, 5.0, 0.0)),
        named(ObjectKind::Group, HIDING, 0x60B4_5523, Some(COOLDOWN), none),
        named(ObjectKind::Image, HIDING_ICON, 0x3807_4901, Some(COOLDOWN), none),
        named(ObjectKind::Group, HEAT, 0xC46A_80A9, None, none),
        object(ObjectKind::Group, SPEEDOMETER, "SpeedometerGroup", None, none),
    ];
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

/// The mirror's group (`0xab9c1d4d`, the radar's group) fades in, and its children with it: past the fade it is at
/// full alpha, as it is in play after its first second.
fn setup() -> (Runtime, PackageId, HudBinding) {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let mut binding = HudBinding::new(&mut rt, id);
    binding.advance_mirror_fade(1.0);
    (rt, id, binding)
}

fn visible(rt: &Runtime, id: PackageId, guid: u32) -> bool {
    rt.tree(id).nodes.iter().find(|n| n.guid == guid).unwrap().visible
}

fn text(rt: &Runtime, id: PackageId, guid: u32) -> String {
    rt.tree(id).nodes.iter().find(|n| n.guid == guid).unwrap().text.clone().unwrap_or_default()
}

fn width(rt: &Runtime, id: PackageId, guid: u32) -> f32 {
    rt.tree(id).nodes.iter().find(|n| n.guid == guid).unwrap().size.x
}

fn chase(secs: f32, evade: f32) -> HudState {
    HudState { pursuit: Some(PursuitHud { secs, evade }), ..Default::default() }
}

#[test]
fn the_pursuit_board_shows_only_while_pursuing() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &HudState::default());
    assert!(!visible(&rt, id, BOARD) && !visible(&rt, id, TIMER), "no chase: the board is hidden");
    b.apply(&mut rt, &chase(3.0, 0.0));
    assert!(visible(&rt, id, BOARD) && visible(&rt, id, TIMER), "a chase shows the board");
    b.apply(&mut rt, &HudState::default());
    assert!(!visible(&rt, id, BOARD) && !visible(&rt, id, TIMER), "the chase over: hidden again");
    assert!(visible(&rt, id, SPEEDOMETER), "the speedometer stays");
}

#[test]
fn the_timer_shows_the_chase_time() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &chase(12.345, 0.0));
    assert_eq!(text(&rt, id, TIMER), "00:12.34");
}

#[test]
fn the_bounty_row_stays_held_hidden_until_the_bounty_is_fed() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &chase(3.0, 0.0));
    assert!(!visible(&rt, id, BOUNTY_ROW), "no bounty in the feed");
    assert!(visible(&rt, id, TIMER), "the timer still shows");
}

#[test]
fn the_heat_meter_stays_held_hidden() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &chase(3.0, 0.5));
    assert!(!visible(&rt, id, HEAT), "its arcs and label are not bound yet");
}

#[test]
fn the_cooldown_bar_shows_from_five_per_cent_evaded_and_fills_with_the_share() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &chase(3.0, 0.04));
    assert!(!visible(&rt, id, COOLDOWN), "below 5 per cent the bar is not shown");
    b.apply(&mut rt, &chase(3.0, 0.5));
    assert!(visible(&rt, id, COOLDOWN) && visible(&rt, id, BAR) && visible(&rt, id, TRACK));
    assert!((width(&rt, id, BAR) - 108.0).abs() < 1e-3, "half evaded is half the 216 width");
    b.apply(&mut rt, &chase(3.0, 1.0));
    assert!((width(&rt, id, BAR) - 216.0).abs() < 1e-3, "all evaded is the full width");
    b.apply(&mut rt, &HudState::default());
    assert!(!visible(&rt, id, COOLDOWN), "no chase: the bar is hidden");
}

#[test]
fn the_cooldown_fill_grows_from_its_left_end() {
    let (mut rt, id, mut b) = setup();
    let left_edge = |rt: &Runtime| {
        let tree = rt.tree(id);
        let n = tree.nodes.iter().find(|n| n.guid == BAR).unwrap();
        n.position.x - n.size.x / 2.0
    };
    b.apply(&mut rt, &chase(3.0, 0.25));
    let at_quarter = left_edge(&rt);
    b.apply(&mut rt, &chase(3.0, 1.0));
    let full = left_edge(&rt);
    assert!((full - at_quarter).abs() < 1e-3, "the left end stays put as the fill grows: {at_quarter} vs {full}");
}

#[test]
fn the_cooldown_bar_shows_on_its_own_flag_without_the_mirror() {
    let (mut rt, id, mut b) = setup();
    b.set_mirror_shown(false);
    b.apply(&mut rt, &chase(3.0, 0.5));
    assert!(!visible(&rt, id, RADAR_BACKING), "the radar is off with the mirror");
    assert!(visible(&rt, id, COOLDOWN), "the bar is its own part, not the radar's");
    assert!(!visible(&rt, id, HIDING) && !visible(&rt, id, HIDING_ICON), "the hiding part is held hidden");
}

#[test]
fn the_cooldown_bar_is_hidden_with_the_mirror_on_before_the_evade() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &HudState::default());
    assert!(visible(&rt, id, RADAR_BACKING), "the radar shows with the mirror");
    assert!(!visible(&rt, id, COOLDOWN), "the radar's group does not bring the bar with it");
}
