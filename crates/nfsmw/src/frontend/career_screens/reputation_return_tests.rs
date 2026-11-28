use super::*;
use crate::career::{Career, CareerData, CareerHandle};
use crate::frontend::career_screens::rep_sheets::playtest::Harness;
use blackbox_feng::runtime::pad;
use std::sync::Arc;

#[test]
#[ignore = "requires installed career packages and tables"]
fn installed_locked_sonny_challenge_stays_hidden_during_back_transition() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|err| err.into_inner());
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut career = Career::new("UIBACK15", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    career.blacklist.enter_bin(15);
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    let overview = screen::SAFE_HOUSE_REPUTATION_OVERVIEW;
    h.open(overview, make(overview, Args { rival: 15, safehouse: true, ..Args::default() }).unwrap());
    assert!(!h.node(BUTTON_GROUP).visible);
    // Replay the Back response's exit fade while the overview is still present. The normal Back action can
    // switch packages immediately, depending on the installed package's responses.
    h.screens.post_to_top(BUTTON_GROUP, 0x97BD_B465);
    for _ in 0..24 {
        h.run(0, 1);
        assert!(!h.node(BUTTON_GROUP).visible, "the unavailable challenge stays hidden through the exit fade");
    }
    h.press(pad::BACK);
    h.run(0, 60);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "requires installed career packages and tables"]
fn batch15_blacklist_child_back_restores_category_and_fresh_entry_resets() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut career = Career::new("UI15", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    career.blacklist.enter_bin(14);
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    let overview = screen::SAFE_HOUSE_REPUTATION_OVERVIEW;
    for (index, sheet) in SHEETS.into_iter().enumerate() {
        let args = Args { rival: 15, safehouse: true, ..Args::default() };
        h.open(overview, make(&overview.to_ascii_lowercase(), args).unwrap());
        for _ in 0..index {
            h.press(pad::RIGHT);
        }
        for _ in 0..2 {
            h.press(pad::ACCEPT);
            h.run(0, 100);
            assert_eq!(h.screens.top(), sheet.screen());
            h.press(pad::BACK);
            h.run(0, 100);
            assert_eq!(h.screens.top(), Some(overview));
            let label = crate::i18n::game_or_error(sheet.label());
            assert!(
                h.tree().nodes.iter().any(|n| n.visible && n.text.as_deref() == Some(label.as_str())),
                "the returned overview must select {sheet:?}"
            );
        }
    }
    let args = Args { rival: 14, safehouse: true, ..Args::default() };
    h.open(overview, make(&overview.to_ascii_lowercase(), args).unwrap());
    h.press(pad::ACCEPT);
    h.run(0, 100);
    assert_eq!(h.screens.top(), Sheet::RaceEvents.screen());
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "requires installed career packages and tables"]
fn installed_bin_15_in_game_overview_keeps_its_twin_on_milestones_return() {
    use crate::input::InputDevice;
    use blackbox_feng::NodeKind;

    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut career = Career::new("UIGAME15", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    career.blacklist.enter_bin(15);
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    let overview = package(true);
    assert_eq!(overview, screen::IN_GAME_REPUTATION_OVERVIEW);
    let args = Args { rival: 15, pause: true, safehouse: true, ..Args::default() };
    h.open(overview, make(overview, args).unwrap());
    let portrait = fe_hash_upper("RIVAL_15_IG");
    assert!(h.screens.assets().texture(portrait).is_some());
    assert!(matches!(h.node(PORTRAIT).kind, NodeKind::Image { texture, .. } if texture == portrait));
    for device in [InputDevice::Keyboard, InputDevice::Xbox] {
        let tree = h.presented(device);
        assert!(tree.nodes.iter().any(|node| node.visible && node.text.as_deref() == Some("Accept")));
        let has_icon = tree.nodes.iter().any(|node| {
            node.visible
                && matches!(node.kind, NodeKind::Image { texture, .. } if texture == crate::ui::input_icons::ATLAS)
        });
        assert_eq!(has_icon, device == InputDevice::Xbox);
    }
    h.press(pad::RIGHT);
    h.press(pad::ACCEPT);
    assert_eq!(h.screens.top(), Some(screen::IN_GAME_MILESTONES));
    h.press(pad::BACK);
    assert_eq!(h.screens.top(), Some(overview));
    let label = crate::i18n::game_or_error(Sheet::Milestones.label());
    assert!(h.tree().nodes.iter().any(|node| node.visible && node.text.as_deref() == Some(label.as_str())));
    CareerHandle::shared().clear();
}

#[test]
#[ignore = "requires installed career menus and data"]
fn pass_a_blacklist_category_accept_has_one_sound_for_pad_and_pointer() {
    let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut career = Career::new("UIA", Arc::new(CareerData::load(&dir).unwrap()), &Default::default()).unwrap();
    career.blacklist.enter_bin(14);
    CareerHandle::shared().set(career);
    CareerHandle::shared().begin_play();
    let mut h = Harness::installed();
    for pointer in [false, true] {
        for (index, sheet) in SHEETS.into_iter().enumerate() {
            let name = screen::SAFE_HOUSE_REPUTATION_OVERVIEW;
            let args = Args { rival: 15, safehouse: true, overview_sheet: index, ..Args::default() };
            h.open(name, make(&name.to_ascii_lowercase(), args).unwrap());
            crate::audio::menu::take();
            if pointer {
                h.click(h.node(OPTION_PARTS[0]).position.truncate());
            }
            if !pointer {
                h.run(pad::ACCEPT, 1);
            }
            let sounds = crate::audio::menu::take();
            assert_eq!(sounds.iter().filter(|&&id| id == 4).count(), 1, "{sheet:?} pointer={pointer}: {sounds:?}");
            h.run(if pointer { 0 } else { pad::ACCEPT }, 20);
            assert_eq!(h.screens.top(), sheet.screen());
            assert!(!crate::audio::menu::take().contains(&4), "held Accept must not replay the sound");
        }
    }
    CareerHandle::shared().clear();
}
