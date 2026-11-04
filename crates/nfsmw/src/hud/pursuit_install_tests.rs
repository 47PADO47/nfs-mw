//! With the install (`NFSMW_GAME_DIR`): the pursuit board's backup row and the helicopter's minimap icon in the real
//! `HUD_SingleRace.fng`, through the scripts and the layout the package authors.

use blackbox_feng::{PackageId, Runtime, fe_hash_upper};
use blackbox_minimap::Orientation;
use blackbox_pursuit::{Fines, Infraction, InfractionLog};
use game_install::GameDir;

use super::MapBlip;
use super::backup_timer::{APPEAR, FLASH};
use super::bind::HudBinding;
use super::blip_styles::{
    CONE_IDLE, CONE_TRACKING, HELICOPTER, HELICOPTER_CONE, HELICOPTER_CONE_ICON, HELICOPTER_GROUP,
};
use super::infractions::{APPEAR as ROW_APPEAR, COUNT, HIDE, ICON, label};
use super::minimap::MinimapBinding;
use super::pursuit::PursuitHud;
use super::state::{HudState, MapPosition};
use crate::ui::{Catalog, UiAssets};

/// The backing, the row's group (label and timer) and the timer.
const BACKING: u32 = 0x8c20_a763;
const ROW: u32 = 0x6a14_4066;
const TIMER: u32 = 0xbee4_4775;
/// The cops group and its engaged count, which sits where the cool-down's hiding icon (`0x38074901`) shows.
const COPS: u32 = 0xe0b1_430b;
const ENGAGED: u32 = 0x8149_18ca;

fn open() -> Option<(GameDir, Runtime, PackageId)> {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR")?).ok()?;
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/InGameC.bun"]);
    let mut rt = Runtime::new();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").expect("installed HUD").clone());
    Some((dir, rt, id))
}

fn step(rt: &mut Runtime, binding: &mut HudBinding, state: &HudState, dt: f32) {
    binding.apply(rt, state);
    rt.update(dt);
    binding.finish_frame(rt, state);
}

fn visible(rt: &Runtime, id: PackageId, hash: u32) -> bool {
    rt.tree(id).nodes.iter().any(|n| n.name_hash == hash && n.visible)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_backup_row_appears_with_its_label_and_time_and_flashes_when_close() {
    let Some((_, mut rt, id)) = open() else { return };
    let mut b = HudBinding::new(&mut rt, id);
    b.advance_mirror_fade(1.0);
    let chase = |eta: f32| HudState {
        pursuit: Some(PursuitHud { secs: 40.0, backup_eta: eta, ..Default::default() }),
        ..Default::default()
    };
    step(&mut rt, &mut b, &chase(0.0), 0.5);
    assert!(!visible(&rt, id, ROW), "no backup called: the row is hidden");
    for _ in 0..10 {
        step(&mut rt, &mut b, &chase(65.0), 0.1);
    }
    let backing = rt.find(id, BACKING).expect("the backing");
    assert_eq!(rt.script_of(backing), Some(APPEAR));
    assert!(visible(&rt, id, ROW) && visible(&rt, id, TIMER), "the backing's appear brings the label and the timer");
    let text = rt.tree(id).nodes.iter().find(|n| n.name_hash == TIMER).and_then(|n| n.text.clone());
    assert_eq!(text.as_deref(), Some("01:05.00"));
    step(&mut rt, &mut b, &chase(9.0), 0.1);
    assert_eq!(rt.script_of(backing), Some(FLASH));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_cop_counters_leave_for_the_cool_down_and_come_back_after_it() {
    let Some((_, mut rt, id)) = open() else { return };
    let mut b = HudBinding::new(&mut rt, id);
    b.advance_mirror_fade(1.0);
    let chase = |cooldown: bool| HudState {
        pursuit: Some(PursuitHud { secs: 40.0, cooldown, ..Default::default() }),
        cops_engaged: 2,
        ..Default::default()
    };
    let run = |rt: &mut Runtime, b: &mut HudBinding, cooldown: bool| {
        for _ in 0..10 {
            step(rt, b, &chase(cooldown), 0.1);
        }
    };
    run(&mut rt, &mut b, false);
    assert!(visible(&rt, id, ENGAGED), "the meter's appear fades the counters in");
    run(&mut rt, &mut b, true);
    assert_eq!(rt.alpha(rt.find(id, COPS).expect("the cops group")), Some(0), "the meter's exit fades them out");
    assert!(!visible(&rt, id, ENGAGED), "nothing under the cool-down bar's hiding icon");
    run(&mut rt, &mut b, false);
    assert!(visible(&rt, id, ENGAGED), "the cool-down's exit fades them back in");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_helicopter_icon_shows_its_picture_and_blades_on_the_minimap() {
    let Some((dir, mut rt, id)) = open() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    let minimap = MinimapBinding::open_city(&rt, id, &dir, &assets);
    assert!(minimap.is_some());
    let mut b = HudBinding::new(&mut rt, id).with_minimap(minimap);
    let player = [0.0, 0.0];
    let state = HudState {
        minimap: Some(MapPosition { position: player, heading: [0.0, 1.0], orientation: Orientation::North }),
        blips: vec![MapBlip { style: HELICOPTER, position: [60.0, 0.0], active: true, heading: Some([1.0, 0.0]) }],
        pursuit: Some(PursuitHud { secs: 40.0, ..Default::default() }),
        ..Default::default()
    };
    step(&mut rt, &mut b, &state, 0.1);
    let tree = rt.tree(id);
    let group = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(HELICOPTER_GROUP)).expect("the group");
    assert!(group.visible, "the group is placed");
    let pictures: Vec<_> = tree.nodes.iter().filter(|n| n.parent == Some(group.index)).collect();
    assert_eq!(pictures.len(), 2, "the helicopter picture and its blades");
    assert!(pictures.iter().all(|n| n.visible), "both show with the group");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_helicopter_cone_shows_with_the_helicopter_and_plays_its_tracking_script() {
    let Some((dir, mut rt, id)) = open() else { return };
    let assets = UiAssets::load(&dir).unwrap();
    let minimap = MinimapBinding::open_city(&rt, id, &dir, &assets);
    let mut b = HudBinding::new(&mut rt, id).with_minimap(minimap);
    let state = |sees: bool| HudState {
        minimap: Some(MapPosition { position: [0.0, 0.0], heading: [0.0, 1.0], orientation: Orientation::North }),
        blips: vec![
            MapBlip { style: HELICOPTER_CONE, position: [60.0, 0.0], active: sees, heading: Some([1.0, 0.0]) },
            MapBlip { style: HELICOPTER, position: [60.0, 0.0], active: true, heading: Some([1.0, 0.0]) },
        ],
        pursuit: Some(PursuitHud { secs: 40.0, ..Default::default() }),
        ..Default::default()
    };
    let cone = rt.find(id, fe_hash_upper(HELICOPTER_CONE_ICON)).expect("the package's line-of-sight cone");
    step(&mut rt, &mut b, &state(true), 0.1);
    assert!(visible(&rt, id, fe_hash_upper(HELICOPTER_CONE_ICON)), "the cone shows with the helicopter");
    assert_eq!(rt.script_of(cone), Some(CONE_TRACKING));
    eprintln!("tracking colour {:?}", rt.colour_rgba(cone));
    step(&mut rt, &mut b, &state(false), 0.1);
    assert_eq!(rt.script_of(cone), Some(CONE_IDLE));
    eprintln!("idle colour {:?}", rt.colour_rgba(cone));
    step(&mut rt, &mut b, &HudState { blips: Vec::new(), ..state(false) }, 0.1);
    assert!(!visible(&rt, id, fe_hash_upper(HELICOPTER_CONE_ICON)), "no helicopter, no cone");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_bounty_row_appears_in_a_chase_with_the_pending_bounty() {
    let Some((_, mut rt, id)) = open() else { return };
    let mut b = HudBinding::new(&mut rt, id);
    b.advance_mirror_fade(1.0);
    let chase = HudState {
        pursuit: Some(PursuitHud { secs: 40.0, bounty: 162_500, ..Default::default() }),
        ..Default::default()
    };
    for _ in 0..5 {
        step(&mut rt, &mut b, &chase, 0.1);
    }
    let row = rt.find(id, 0x8674_e6d4).expect("the bounty row");
    assert_eq!(rt.script_of(row), Some(APPEAR), "the row plays its authored appear");
    assert!(visible(&rt, id, 0x8674_e6d4) && visible(&rt, id, 0x875e_92eb), "the row and its number show");
    let text = rt.tree(id).nodes.iter().find(|n| n.name_hash == 0x875e_92eb).and_then(|n| n.text.clone());
    assert_eq!(text.as_deref(), Some("162,500"));
}

/// The text the tree shows under `group` (both strings of an infraction row).
fn texts_under(rt: &Runtime, id: PackageId, group: u32) -> Vec<Option<String>> {
    let tree = rt.tree(id);
    let Some(parent) = tree.nodes.iter().find(|n| n.name_hash == group).map(|n| n.index) else { return Vec::new() };
    tree.nodes.iter().filter(|n| n.parent == Some(parent)).map(|n| n.text.clone()).collect()
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn each_new_infraction_pops_a_row_with_its_label_and_the_count_then_the_notifier_leaves() {
    let Some((_, mut rt, id)) = open() else { return };
    let speeding = label(Infraction::Speeding);
    rt.set_string_resolver(move |l| (l == speeding).then(|| "Excessive Speeding".to_owned()));
    let mut b = HudBinding::new(&mut rt, id);
    b.advance_mirror_fade(1.0);
    let fines = Fines::new([150, 350, 1000, 350, 300, 100, 300, 75]);
    let chase = |log: InfractionLog| HudState {
        pursuit: Some(PursuitHud { secs: 5.0, infractions: log, ..Default::default() }),
        ..Default::default()
    };
    let mut log = InfractionLog::default();
    step(&mut rt, &mut b, &chase(log), 0.1);
    let rows: Vec<_> =
        (1..=4).map(|n| rt.find(id, fe_hash_upper(&format!("INFRACTION_GROUP_{n}"))).expect("a row")).collect();
    assert!(rows.iter().all(|row| rt.script_of(*row) == Some(HIDE)), "the rows rest hidden");
    log.record(Infraction::Speeding, &fines);
    for _ in 0..4 {
        step(&mut rt, &mut b, &chase(log), 0.1);
    }
    assert_eq!(rt.script_of(rows[0]), Some(ROW_APPEAR), "the first row shows the new infraction");
    assert!(visible(&rt, id, fe_hash_upper("INFRACTION_GROUP_1")), "and is drawn");
    let row_texts = texts_under(&rt, id, fe_hash_upper("INFRACTION_GROUP_1"));
    assert_eq!(row_texts.len(), 2, "the text and its shadow");
    assert!(row_texts.iter().all(|t| t.as_deref() == Some("Excessive Speeding")), "{row_texts:?}");
    let count = rt.tree(id).nodes.iter().find(|n| n.name_hash == fe_hash_upper(COUNT)).and_then(|n| n.text.clone());
    assert_eq!(count.as_deref(), Some("1"));
    assert!(visible(&rt, id, fe_hash_upper(ICON)) && visible(&rt, id, fe_hash_upper(COUNT)), "the icon and the count");
    log.record(Infraction::OffRoad, &fines);
    step(&mut rt, &mut b, &chase(log), 0.1);
    assert_eq!(rt.script_of(rows[1]), Some(ROW_APPEAR), "the next infraction takes the next row");
    for _ in 0..40 {
        step(&mut rt, &mut b, &chase(log), 0.1);
    }
    assert!(rows.iter().all(|row| rt.script_of(*row) == Some(HIDE)), "each row left by its own chain");
    assert!(
        !visible(&rt, id, fe_hash_upper(ICON)) && !visible(&rt, id, fe_hash_upper(COUNT)),
        "the icon left with them"
    );
}
