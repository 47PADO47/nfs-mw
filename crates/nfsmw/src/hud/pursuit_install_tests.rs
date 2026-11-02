//! With the install (`NFSMW_GAME_DIR`): the helicopter's minimap icon in the real `HUD_SingleRace.fng`, through the
//! layout the package authors.

use blackbox_feng::{PackageId, Runtime, fe_hash_upper};
use blackbox_minimap::Orientation;
use game_install::GameDir;

use super::MapBlip;
use super::bind::HudBinding;
use super::blip_styles::{HELICOPTER, HELICOPTER_GROUP};
use super::minimap::MinimapBinding;
use super::pursuit::PursuitHud;
use super::state::{HudState, MapPosition};
use crate::ui::{Catalog, UiAssets};

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
