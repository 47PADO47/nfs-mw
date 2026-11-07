//! The pursuit board and the cool-down bar, bound to a small package with the objects of the real one (names and
//! parents as in `HUD_SingleRace.fng`, see `docs/specs/hud-pursuit.md`).

use blackbox_feng::package::{
    Event, Interp, Key, MessageTargets, ObjectData, ObjectDef, ObjectKind, Package, ParamType, Response, ResponseKind,
    ResponseList, ResponseParam, Script, StringDef, Track,
};
use blackbox_feng::{PackageId, Runtime, fe_hash_upper};
use glam::Vec3;

use super::bind::HudBinding;
use super::pursuit::PursuitHud;
use super::state::HudState;

/// The pursuit board's group, its timer, its bounty row and the row's number, and the radar's group with the cool-down
/// bar under it.
const BOARD: u32 = 1;
const TIMER: u32 = 2;
const BOUNTY_ROW: u32 = 3;
const BOUNTY_VALUE: u32 = 13;
const RADAR: u32 = 4;
const RADAR_BACKING: u32 = 12;
const COOLDOWN: u32 = 5;
const BAR: u32 = 6;
const TRACK: u32 = 7;
const HIDING: u32 = 8;
const HIDING_ICON: u32 = 9;
/// The orange bar the board drives: its appear and leave raise the messages the mark answers.
const HIDING_BAR: u32 = 16;
const MARK_IN: u32 = 0x4027_A94E;
const MARK_OUT: u32 = 0x451C_D24C;
const HEAT: u32 = 10;
const SPEEDOMETER: u32 = 11;
/// The cost-to-state flasher's group and its number.
const CTS: u32 = 14;
const CTS_VALUE: u32 = 15;

fn named(kind: ObjectKind, guid: u32, name_hash: u32, parent: Option<u32>, size: Vec3) -> ObjectDef {
    let mut words = vec![0u32; 17];
    for word in words.iter_mut().take(4) {
        *word = 255;
    }
    // The hiding mark rests at zero alpha, as in the package; its scripts fade it in and out.
    if matches!(name_hash, 0x60B4_5523 | 0x3807_4901) {
        words[3] = 0;
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
        scripts: match name_hash {
            0x84A2_26EC | 0x839E_7D77 => visibility_scripts(),
            0x60B4_5523 | 0x3807_4901 => fade_scripts(),
            0x5C2A_4F20 => bar_scripts(),
            _ => Vec::new(),
        },
        responses: match name_hash {
            0x60B4_5523 | 0x3807_4901 => mark_responses(),
            _ => Vec::new(),
        },
    }
}

/// The bar's appear and leave: each raises its message to the package at once, as the installed ones do.
fn bar_scripts() -> Vec<Script> {
    [(super::hiding_mark::APPEAR, MARK_IN), (0x0331_13AC, MARK_OUT)]
        .into_iter()
        .map(|(id, message)| Script {
            id,
            events: vec![Event { message, target: 0xFFFF_FFFC, time: 0 }],
            ..Default::default()
        })
        .collect()
}

/// The mark's answers: the bar's appear fades it in, its leave out.
fn mark_responses() -> Vec<ResponseList> {
    [(MARK_IN, 1_350_158_584), (MARK_OUT, 53_547_948)]
        .into_iter()
        .map(|(message, script)| ResponseList {
            message,
            responses: vec![Response {
                kind: ResponseKind::SetScript,
                param: ResponseParam::Number(script),
                target: 0,
            }],
        })
        .collect()
}

/// The mark's fade-in and fade-out: the installed script ids, setting the alpha outright.
fn fade_scripts() -> Vec<Script> {
    [(1_350_158_584, 255), (53_547_948, 0)]
        .into_iter()
        .map(|(id, alpha)| Script {
            id,
            tracks: vec![Track {
                param: ParamType::Colour,
                interp: Interp::Linear,
                action: 0,
                length: 0,
                offset: 0,
                base: Key { time: -1, value: [255, 255, 255, alpha] },
                keys: Vec::new(),
            }],
            ..Default::default()
        })
        .collect()
}

pub(super) fn visibility_scripts() -> Vec<Script> {
    [
        (super::radar::HIDE, 0),
        (super::radar::APPEAR, 255),
        (super::radar::LEAVE, 0),
        (0x9297_5065, 0),
        (0x13F5_1124, 255),
    ]
    .into_iter()
    .map(|(id, alpha)| Script {
        id,
        tracks: vec![Track {
            param: ParamType::Colour,
            interp: Interp::Linear,
            action: 0,
            length: 0,
            offset: 0,
            base: Key { time: -1, value: [255, 255, 255, alpha] },
            keys: Vec::new(),
        }],
        ..Default::default()
    })
    .collect()
}

fn step(rt: &mut Runtime, binding: &mut HudBinding, state: &HudState) {
    binding.apply(rt, state);
    rt.update(1.0);
    binding.finish_frame(rt, state);
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
        named(ObjectKind::String, BOUNTY_VALUE, 0x875E_92EB, Some(BOUNTY_ROW), none),
        // The radar's group (`0xab9c1d4d`) holds its backing (the layout's radar object) and the cool-down bar.
        named(ObjectKind::Group, RADAR, 0xAB9C_1D4D, None, none),
        named(ObjectKind::Image, RADAR_BACKING, 0x839E_7D77, Some(RADAR), none),
        named(ObjectKind::Group, COOLDOWN, 0x84A2_26EC, Some(RADAR), none),
        named(ObjectKind::Image, BAR, 0xCF81_7638, Some(COOLDOWN), Vec3::new(216.0, 5.0, 0.0)),
        named(ObjectKind::Image, TRACK, 0xA1EB_7E6E, Some(COOLDOWN), Vec3::new(216.0, 5.0, 0.0)),
        named(ObjectKind::Group, HIDING, 0x60B4_5523, Some(COOLDOWN), none),
        named(ObjectKind::Image, HIDING_ICON, 0x3807_4901, Some(COOLDOWN), none),
        named(ObjectKind::Image, HIDING_BAR, super::hiding_mark::BAR, Some(COOLDOWN), none),
        named(ObjectKind::Group, HEAT, 0xC46A_80A9, None, none),
        named(ObjectKind::String, 20, 0xA81A_2028, Some(HEAT), none),
        named(ObjectKind::String, 21, 0xA81A_2028, Some(HEAT), none),
        named(ObjectKind::String, 22, 0x8149_18CA, Some(BOARD), none),
        named(ObjectKind::String, 23, 0x5FA7_0D4C, Some(BOARD), none),
        named(ObjectKind::Image, 24, 0x2419_6AD4, Some(RADAR), none),
        object(ObjectKind::Group, SPEEDOMETER, "SpeedometerGroup", None, none),
        object(ObjectKind::Group, CTS, "CTS_GROUP", None, none),
        named(ObjectKind::String, CTS_VALUE, 0x3FF5_F33C, Some(CTS), none),
    ];
    Package {
        name: "HUD.fng".into(),
        file_name: String::new(),
        version: 0x20000,
        resources: Vec::new(),
        objects,
        button_count: 0,
        responses: Vec::new(),
        targets: [MARK_IN, MARK_OUT].map(|message| MessageTargets { message, guids: vec![HIDING, HIDING_ICON] }).into(),
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
    HudState {
        pursuit: Some(PursuitHud { secs, evade, cooldown: evade >= 0.05, ..Default::default() }),
        ..Default::default()
    }
}

#[test]
fn the_pursuit_board_shows_only_while_pursuing() {
    let (mut rt, id, mut b) = setup();
    step(&mut rt, &mut b, &HudState::default());
    assert!(!visible(&rt, id, BOARD) && !visible(&rt, id, TIMER), "no chase: the board is hidden");
    step(&mut rt, &mut b, &chase(3.0, 0.0));
    assert!(visible(&rt, id, BOARD) && visible(&rt, id, TIMER), "a chase shows the board");
    step(&mut rt, &mut b, &HudState::default());
    assert!(!visible(&rt, id, BOARD) && !visible(&rt, id, TIMER), "the chase over: hidden again");
    assert!(visible(&rt, id, SPEEDOMETER), "the speedometer stays");
}

#[test]
fn the_timer_shows_the_chase_time() {
    let (mut rt, id, mut b) = setup();
    step(&mut rt, &mut b, &chase(12.345, 0.0));
    assert_eq!(text(&rt, id, TIMER), "00:12.34");
}

#[test]
fn the_bounty_row_shows_with_the_board_and_reads_the_pending_bounty() {
    let (mut rt, id, mut b) = setup();
    let mut state = chase(3.0, 0.0);
    state.pursuit.as_mut().unwrap().bounty = 12_500;
    step(&mut rt, &mut b, &state);
    assert!(visible(&rt, id, BOUNTY_ROW) && visible(&rt, id, BOUNTY_VALUE), "the row shows in every chase");
    assert_eq!(text(&rt, id, BOUNTY_VALUE), "12,500", "the pending bounty with the thousands separator");
    assert!(visible(&rt, id, TIMER), "the timer still shows");
    step(&mut rt, &mut b, &HudState::default());
    assert!(!visible(&rt, id, BOUNTY_ROW), "no chase: the row hides with the board");
}

#[test]
fn a_rise_of_the_cost_to_state_flashes_its_total_for_two_seconds() {
    // `setup` runs the frames one second apart.
    let (mut rt, id, mut b) = setup();
    step(&mut rt, &mut b, &chase(3.0, 0.0));
    assert!(!visible(&rt, id, CTS), "no cost yet: the flasher rests hidden");
    let mut state = chase(4.0, 0.0);
    state.police_intel.cost_to_state = 2_250;
    step(&mut rt, &mut b, &state);
    assert!(visible(&rt, id, CTS) && visible(&rt, id, CTS_VALUE), "a rise shows the total");
    assert_eq!(text(&rt, id, CTS_VALUE), "2,250");
    for _ in 0..4 {
        step(&mut rt, &mut b, &state);
    }
    assert!(!visible(&rt, id, CTS), "the same total does not show again: the flasher has played out");
}

#[test]
fn the_heat_meter_stays_held_hidden() {
    let (mut rt, id, mut b) = setup();
    step(&mut rt, &mut b, &chase(3.0, 0.5));
    assert!(!visible(&rt, id, HEAT), "its arcs and label are not bound yet");
}

#[test]
fn session_heat_drives_both_duplicate_labels_and_counters_replace_placeholders() {
    let (mut rt, id, mut binding) = setup();
    let state = HudState { vehicle_heat: 3.8, cops_engaged: 4, cops_damaged: 2, ..chase(10.0, 0.0) };
    step(&mut rt, &mut binding, &state);
    // This HUD has no minimap, and the heat meter rings the minimap: it is hidden, but its labels are still fed.
    assert!(!visible(&rt, id, HEAT), "no minimap, no heat ring");
    assert_eq!(text(&rt, id, 20), "x3");
    assert_eq!(text(&rt, id, 21), "x3");
    assert_eq!(text(&rt, id, 22), "4");
    assert_eq!(text(&rt, id, 23), "2");
    step(&mut rt, &mut binding, &HudState { vehicle_heat: 2.5, ..Default::default() });
    assert_eq!(text(&rt, id, 20), "x2", "vehicle heat survives outside a pursuit");
    assert_eq!(text(&rt, id, 21), "x2");
}

#[test]
fn radar_direction_is_hidden_without_a_target_and_shows_for_a_nearby_patrol() {
    let (mut rt, id, mut binding) = setup();
    step(&mut rt, &mut binding, &HudState::default());
    assert!(!visible(&rt, id, 24));
    step(
        &mut rt,
        &mut binding,
        &HudState {
            radar: Some(super::RadarHud { angle: 0.5, distance: 30.0 }),
            cop_nearby: true,
            ..Default::default()
        },
    );
    assert!(visible(&rt, id, 24));
    step(&mut rt, &mut binding, &chase(3.0, 0.0));
    assert!(!visible(&rt, id, 24));
}

#[test]
fn the_cooldown_phase_shows_the_bar_and_fills_with_the_supplied_share() {
    let (mut rt, id, mut b) = setup();
    step(&mut rt, &mut b, &chase(3.0, 0.04));
    assert!(!visible(&rt, id, COOLDOWN), "before cooldown the bar is not shown");
    step(&mut rt, &mut b, &chase(3.0, 0.5));
    assert!(visible(&rt, id, COOLDOWN) && visible(&rt, id, BAR) && visible(&rt, id, TRACK));
    assert!((width(&rt, id, BAR) - 108.0).abs() < 1e-3, "half evaded is half the 216 width");
    step(&mut rt, &mut b, &chase(3.0, 1.0));
    assert!((width(&rt, id, BAR) - 216.0).abs() < 1e-3, "all evaded is the full width");
    step(&mut rt, &mut b, &HudState::default());
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
    step(&mut rt, &mut b, &chase(3.0, 0.25));
    let at_quarter = left_edge(&rt);
    step(&mut rt, &mut b, &chase(3.0, 1.0));
    let full = left_edge(&rt);
    assert!((full - at_quarter).abs() < 1e-3, "the left end stays put as the fill grows: {at_quarter} vs {full}");
}

#[test]
fn the_cooldown_bar_shows_on_its_own_flag_without_the_mirror() {
    let (mut rt, id, mut b) = setup();
    b.set_mirror_shown(false);
    step(&mut rt, &mut b, &chase(3.0, 0.5));
    assert!(visible(&rt, id, RADAR_BACKING), "cooldown radar remains available without the mirror");
    assert!(visible(&rt, id, COOLDOWN), "the bar is its own part, not the radar's");
    assert!(!visible(&rt, id, HIDING) && !visible(&rt, id, HIDING_ICON), "no spot hides the player: the mark rests");
}

#[test]
fn the_hiding_mark_fades_in_while_a_spot_latches_the_player_and_out_when_it_lets_go() {
    let (mut rt, id, mut b) = setup();
    b.set_mirror_shown(false);
    let mut state = chase(3.0, 0.5);
    step(&mut rt, &mut b, &state);
    state.pursuit.as_mut().unwrap().hiding = Some(1.0);
    step(&mut rt, &mut b, &state);
    step(&mut rt, &mut b, &state);
    assert!(
        visible(&rt, id, HIDING) && visible(&rt, id, HIDING_ICON),
        "the mark shows while the spot hides the player"
    );
    state.pursuit.as_mut().unwrap().hiding = None;
    step(&mut rt, &mut b, &state);
    step(&mut rt, &mut b, &state);
    assert!(!visible(&rt, id, HIDING) && !visible(&rt, id, HIDING_ICON), "and goes when the spot lets go");
}

#[test]
fn the_hiding_mark_stays_down_outside_the_cool_down() {
    let (mut rt, id, mut b) = setup();
    let mut state = chase(3.0, 0.0);
    state.pursuit.as_mut().unwrap().hiding = Some(1.0);
    step(&mut rt, &mut b, &state);
    step(&mut rt, &mut b, &state);
    let bar = rt.find(id, super::hiding_mark::BAR).unwrap();
    assert_ne!(rt.script_of(bar), Some(super::hiding_mark::APPEAR), "the board drives the bar only in the cool-down");
    assert!(!visible(&rt, id, HIDING) && !visible(&rt, id, HIDING_ICON));
}

#[test]
fn the_cooldown_bar_is_hidden_with_the_mirror_on_before_the_evade() {
    let (mut rt, id, mut b) = setup();
    step(&mut rt, &mut b, &HudState::default());
    assert!(!visible(&rt, id, RADAR_BACKING), "the mirror supplies its own detector backing");
    assert!(!visible(&rt, id, COOLDOWN), "the radar's group does not bring the bar with it");
}
