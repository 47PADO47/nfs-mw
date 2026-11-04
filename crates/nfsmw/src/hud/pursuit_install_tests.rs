//! With the install (`NFSMW_GAME_DIR`): the pursuit board's backup row and the helicopter's minimap icon in the real
//! `HUD_SingleRace.fng`, through the scripts and the layout the package authors.

use blackbox_feng::{PackageId, Runtime, fe_hash_upper};
use blackbox_minimap::Orientation;
use game_install::GameDir;

use super::MapBlip;
use super::backup_timer::{APPEAR, FLASH};
use super::bind::HudBinding;
use super::blip_styles::{
    CONE_IDLE, CONE_TRACKING, HELICOPTER, HELICOPTER_CONE, HELICOPTER_CONE_ICON, HELICOPTER_GROUP,
};
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
