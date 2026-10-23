//! Regressions for the native carousel's pointer selection and modal return.

use super::*;
use crate::frontend::icon_items::{Id, icons};
use crate::frontend::icon_menu::Kind;
use blackbox_feng::NodeKind;

fn icon_centre(h: &Harness, texture: u32) -> glam::Vec2 {
    let tree = h.screens.trees().pop().unwrap();
    tree.nodes
        .iter()
        .find(|n| n.visible && matches!(n.kind, NodeKind::Image { texture: t, .. } if t == texture))
        .expect("the target icon is visible")
        .world
        .transform_point3(glam::Vec3::ZERO)
        .truncate()
}

fn assert_carousel_header_visible(h: &Harness) {
    let tree = h.screens.trees().pop().unwrap();
    let header = tree.nodes.iter().find(|node| node.name_hash == crate::frontend::ids::ICON_TITLE).expect("icon title");
    assert!(header.visible && header.world_colour[3] > 0, "returning from a dialog must preserve the carousel header");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn clicking_an_unselected_carousel_icon_opens_that_icon() {
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
    h.wait(1.0);
    // Career is selected. Quick Race is visible beside it and uses the same submenu package,
    // so verify its contents rather than accepting any switch to MainMenu_Sub as success.
    let target = icons(Kind::Main).into_iter().find(|icon| icon.id == Id::QuickRace).unwrap();
    let point = icon_centre(&h, target.texture);
    assert!(h.click(point), "the carousel takes its own icon click");
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    let tree = h.screens.trees().pop().unwrap();
    let quick_play = icons(Kind::QuickRace).into_iter().find(|icon| icon.id == Id::QuickPlay).unwrap();
    assert!(
        tree.nodes.iter().any(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == quick_play.texture))
    );
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn clicking_a_windowed_category_opens_its_bound_action() {
    let args = Args { options: true, ..Args::default() };
    let mut h = Harness::open(screen::MAIN_MENU_SUB, args).expect("installed menus");
    h.wait(1.0);
    let list = icons(Kind::Categories { pause: false, mods: false });
    let controls = list.iter().position(|icon| icon.id == Id::ControlOptions).unwrap();
    for _ in 0..controls {
        h.press(pad::RIGHT);
    }
    h.wait(0.5);
    let video = list.iter().find(|icon| icon.id == Id::Category(Category::Player)).unwrap();
    assert!(h.click(icon_centre(&h, video.texture)));
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::OPTIONS));
    h.press(pad::RIGHT);
    assert!(h.changed.transmission.is_some(), "the Player category opens, rather than the selected Controls category");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn a_native_carousel_can_reopen_a_question_after_no_or_back() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    let dir =
        GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").expect("installed game"))).unwrap();
    let data = Arc::new(crate::career::CareerData::load(&dir).unwrap());
    let career = crate::career::Career::new("menus", data, &nfsmw_career::settings::NewCareer::default()).unwrap();
    let handle = crate::career::CareerHandle::shared();
    handle.set(career);
    handle.set_prelude(true);
    for answer in [Some(pad::ACCEPT), Some(pad::BACK), None] {
        let args = Args { safehouse: true, ..Args::default() };
        let mut h = Harness::open(screen::MAIN_MENU_SUB, args).expect("installed menus");
        h.wait(1.0);
        assert_carousel_header_visible(&h);
        h.press(pad::ACCEPT);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(screen::DIALOG));
        match answer {
            Some(button) => h.press(button),
            None => {
                let no = centre_of(&h, crate::frontend::confirm::NO_BUTTON).expect("the No answer");
                assert!(h.click(no));
            }
        }
        h.wait(1.0);
        assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
        assert_carousel_header_visible(&h);
        assert!(!h.said(&Command::Resume));
        // Go through the actual SafeHouseMenu wrapper's mouse forwarding on the second attempt.
        let resume =
            crate::frontend::icon_items::safehouse_icons(&[nfsmw_career::safehouse::menu::MenuItem::ResumeFreeRoam])[0];
        assert!(h.click(icon_centre(&h, resume.texture)));
        h.wait(1.5);
        assert_eq!(
            h.screens.top(),
            Some(screen::DIALOG),
            "the retained carousel must accept another gesture after {answer:?}"
        );
    }
    // Alias Save retains its carousel while pushing a question, rather than switching screens.
    handle.end_play();
    let mut h = Harness::open(screen::MC_PROFILE_MANAGER, Args::default()).expect("installed menus");
    h.wait(1.0);
    assert_carousel_header_visible(&h);
    let aliases = icons(Kind::AliasManager);
    let save = aliases.iter().position(|icon| icon.id == Id::AliasSave).unwrap();
    for _ in 0..save {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::DIALOG));
    h.press(pad::ACCEPT);
    h.wait(0.5);
    assert_eq!(h.screens.top(), Some(screen::MC_PROFILE_MANAGER));
    assert_carousel_header_visible(&h);
    assert!(!h.said(&Command::CareerSave));
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "Alias Save must react after No too");
    h.press(pad::BACK);
    h.wait(0.5);
    h.press(pad::LEFT);
    h.press(pad::ACCEPT);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::MC_LIST), "Alias Load pushes the profile list");
    h.press(pad::BACK);
    h.wait(1.0);
    assert_eq!(h.screens.top(), Some(screen::MC_PROFILE_MANAGER));
    assert_carousel_header_visible(&h);
    handle.clear();
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn pause_quit_and_restart_still_react_after_no_and_cancel() {
    for race in [false, true] {
        for answer in [pad::ACCEPT, pad::BACK] {
            let args = Args {
                pause: true,
                pause_ctx: crate::frontend::logic::PauseCtx { race, ..Args::default().pause_ctx },
                ..Args::default()
            };
            let targets = crate::frontend::icon_items::pause_icons(args.pause_ctx);
            for id in [Id::RestartRace, Id::Quit(args.pause_ctx.quit)] {
                let Some(index) = targets.iter().position(|icon| icon.id == id) else { continue };
                let mut h = Harness::open(screen::PAUSE_MENU, args).expect("installed menus");
                h.wait(1.0);
                for _ in 0..index {
                    h.press(pad::RIGHT);
                }
                h.press(pad::ACCEPT);
                h.wait(0.5);
                assert_eq!(h.screens.top(), Some(screen::DIALOG));
                h.press(answer);
                h.wait(0.5);
                assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU));
                assert!(!h.said(&Command::RestartRace));
                h.press(pad::START);
                h.wait(1.5);
                assert!(h.said(&Command::Resume), "pause menu must resume after {id:?}, answer {answer}");
            }
        }
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn custom_options_can_change_rows_and_reopen_defaults_after_no() {
    let _state = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
    use crate::frontend::custom_race::{Mode, state};
    state::with(|state| {
        state.split.end();
        state.choose_mode(Mode::Circuit);
    });
    let mut h = Harness::open(screen::TRACK_OPTIONS, Args::default()).expect("installed menus");
    h.wait(1.0);
    let defaults = pad::BUTTON0 << 5;
    h.press(defaults);
    h.wait(0.5);
    assert_eq!(h.screens.top(), Some(screen::DIALOG));
    h.press(pad::ACCEPT);
    h.wait(0.5);
    assert_eq!(h.screens.top(), Some(screen::TRACK_OPTIONS));
    assert!(!h.said(&Command::CustomDefaults));
    let before = state::with(|state| *state.settings());
    h.press(pad::DOWN);
    h.press(pad::RIGHT);
    assert_ne!(state::with(|state| *state.settings()), before, "No must restore option row input");
    h.press(defaults);
    h.wait(0.5);
    assert_eq!(h.screens.top(), Some(screen::DIALOG), "No must return control to race options");
}
