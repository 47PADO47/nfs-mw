//! Installed profile rows use their authored highlight and the alias under the pointer.

use super::*;
use blackbox_feng::Runtime;
use std::collections::HashMap;

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn native_alias_click_selects_the_clicked_row_of_the_current_page() {
    use crate::frontend::logic::Env;
    use crate::settings::{Partial, Settings, test_caps};
    use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

    let dir = game_install::GameDir::open(std::path::PathBuf::from(
        std::env::var_os("NFSMW_GAME_DIR").expect("installed menus"),
    ))
    .unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let mut rt = Runtime::new();
    let package = rt.load(catalog.find(screen::MC_LIST).unwrap().clone());
    for _ in 0..120 {
        rt.update(1.0 / 60.0);
    }
    let mut settings = Settings::from(Partial::default());
    let mut changed = Partial::default();
    let mut commands = Vec::new();
    let mut memory = HashMap::new();
    let env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
    let mut cx = Cx {
        rt: &mut rt,
        package,
        assets: &assets,
        settings: env.settings,
        changed: env.changed,
        caps: env.caps,
        commands: &mut commands,
        memory: &mut memory,
        key_texts: &[],
        world_map: None,
        mods: None,
        name: screen::MC_LIST,
    };
    let mut list = AliasListLogic {
        args: Args::default(),
        mode: ListMode::Load,
        aliases: (0..12).map(|i| format!("A{i}")).collect(),
        selected: 8,
    };
    list.show(&mut cx);
    cx.rt.update(0.0);
    let target = cx.named("option_name_2").unwrap();
    assert!(!cx.rt.buttons(package).contains(&target), "profile text does not depend on button focus");
    assert!(
        cx.rt.package(package).unwrap().objects[target.index].scripts.iter().any(|s| s.id == SCRIPT_HIGHLIGHT),
        "the installed row supplies its authored highlight"
    );
    let at = cx.rt.object_quad(target).unwrap().transform_point3(glam::Vec3::new(100.0, 0.0, 0.0)).truncate();
    list.hover(&mut cx, at);
    assert_eq!(list.selected, 9, "pointer hover maps the second page's second slot to A9");
    cx.rt.update(1.0 / 60.0);
    assert_eq!(cx.rt.script_of(target), Some(SCRIPT_HIGHLIGHT));
    let cursor = cx.guid(0x000F_61C2).expect("authored profile selector");
    assert_eq!(cx.rt.script_of(cursor), Some(0x001B_20C3), "second-slot highlight moves the authored brackets");
    assert!(list.click(&mut cx, at));
    assert_eq!(cx.commands.as_slice(), &[Command::CareerLoad("A9".into())]);
    let tree = cx.rt.tree(package);
    for (slot, expected) in ["A8", "A9", "A10", "A11"].into_iter().enumerate() {
        let name = blackbox_feng::fe_hash_upper(&format!("option_name_{}", slot + 1));
        assert_eq!(tree.nodes.iter().find(|n| n.name_hash == name).unwrap().text.as_deref(), Some(expected));
    }
    cx.commands.clear();
    list.pad_pressed(&mut cx, pad::ACCEPT);
    assert_eq!(
        cx.commands.as_slice(),
        &[Command::CareerLoad("A9".into())],
        "keyboard/controller acceptance remains active"
    );
    cx.commands.clear();
    let empty = cx.named("option_name_8").unwrap();
    let at = cx.rt.object_quad(empty).unwrap().transform_point3(glam::Vec3::new(100.0, 0.0, 0.0)).truncate();
    assert!(list.click(&mut cx, at));
    assert!(cx.commands.is_empty(), "an empty row cannot load the previous selection");
}
