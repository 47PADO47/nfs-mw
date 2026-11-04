use super::*;

#[test]
fn career_batch13_blacklist_race_category_uses_crossed_flags() {
    assert_eq!(texture(Sheet::RaceEvents), 0xefc9_662e);
    assert_ne!(texture(Sheet::RaceEvents), fe_hash_upper("MAIN_ICON_REP_SHEET"));
}

#[test]
#[ignore = "requires installed Blacklist artwork; set NFSMW_GAME_DIR"]
fn career_batch13_blacklist_category_art_is_loaded_at_native_resolution() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let assets = crate::ui::UiAssets::load(&dir).unwrap();
    let flags = assets.texture(texture(Sheet::RaceEvents)).expect("installed Race Events crossed flags");
    assert_eq!(flags.name, "RACE_EVENTS");
    assert!(flags.width >= 64 && flags.width == flags.height, "stock or higher-resolution square flags");
    let rgba = blackbox_tpk::decode_rgba8(&flags).expect("decode installed flags without resampling");
    assert_eq!(rgba.len(), flags.width as usize * flags.height as usize * 4);
    eprintln!("RACE_EVENTS: installed {}x{}", flags.width, flags.height);
}

#[test]
#[ignore = "requires installed Blacklist package and career tables; set NFSMW_GAME_DIR"]
fn career_batch13_blacklist_category_keeps_its_flags_when_selection_and_rival_change() {
    use crate::career::{Career, CareerData, CareerHandle};
    use crate::frontend::career_screens::{rep_sheets::playtest::Harness, reputation_overview};
    use crate::frontend::{ids::screen, logic::Args};
    use blackbox_feng::{NodeKind, runtime::pad};
    use std::sync::Arc;

    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut career = Career::new("UI13", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    career.blacklist.enter_bin(14);
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    let name = screen::SAFE_HOUSE_REPUTATION_OVERVIEW;
    h.open(name, reputation_overview::make(&name.to_ascii_lowercase(), Args::default()).unwrap());
    let assert_flags = |h: &Harness| {
        assert!(h.tree().nodes.iter().any(|node| node.visible
            && matches!(node.kind, NodeKind::Image {texture: got, ..} if got == texture(Sheet::RaceEvents))));
    };
    assert_flags(&h);
    for direction in [pad::RIGHT, pad::UP, pad::LEFT, pad::DOWN] {
        h.press(direction);
        assert_flags(&h);
    }
    h.press(pad::BACK);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    CareerHandle::shared().clear();
}
