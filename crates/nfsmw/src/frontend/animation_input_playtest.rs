//! Rejected input during the actual installed icon-menu exit must not sound or restart the transition.

use super::*;
use crate::audio::menu;

#[test]
#[ignore = "requires installed track menus; set NFSMW_GAME_DIR"]
fn batch12_track_accept_exit_rejects_repeated_native_navigation_sounds() {
    use super::custom_race_screen_tests::{TURN, open_in};
    use crate::frontend::custom_race::{Mode, state, tracks};

    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|error| error.into_inner());
    let _turn = TURN.lock().unwrap_or_else(|error| error.into_inner());
    let mut h = open_in(Mode::Circuit, screen::TRACK_SELECT, Args::default()).expect("installed track menu");
    let board = crate::frontend::quick_race::board().unwrap();
    let standing = crate::career::CareerHandle::shared()
        .with_loaded(|career| tracks::Standing::of(&career.blacklist))
        .unwrap_or_else(|| tracks::Standing::at(crate::frontend::quick_play::pool::PROLOGUE_STAGE));
    let listed =
        tracks::list(&board, Mode::Circuit, state::with(|state| state.filter), &standing, tracks::unlock_all());
    let open = listed.iter().position(|track| !track.locked).expect("an open circuit");
    for _ in 0..open {
        h.press(pad::RIGHT);
    }
    h.run(pad::ACCEPT, 1);
    h.run(0, 1);
    let accepted = state::with(|state| state.event().map(str::to_owned));
    assert!(accepted.is_some(), "the test must enter the native Accept exit");
    menu::take();
    for bits in [pad::LEFT, pad::RIGHT, pad::LTRIGGER, pad::RTRIGGER, pad::ACCEPT] {
        h.run(bits, 1);
        h.run(0, 1);
    }
    assert!(menu::take().is_empty(), "rejected track input must not emit native navigation sounds");
    assert_eq!(state::with(|state| state.event().map(str::to_owned)), accepted);
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::TRACK_OPTIONS));
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn batch12_settings_back_exit_blocks_pointer_fallback_raw_posts_and_feedback() {
    for (name, pause) in [(screen::OPTIONS, false), (screen::PAUSE_OPTIONS, true)] {
        let mut control = None;
        for inject in [false, true] {
            let mut h = Harness::open(name, Args { pause, category: Category::Audio, ..Default::default() })
                .expect("installed menus");
            h.wait(1.0);
            let button = h.screens.top_buttons().into_iter().next().expect("native settings button");
            let tree = h.screens.trees().pop().unwrap();
            let point = tree
                .nodes
                .iter()
                .find(|node| node.index == button.index)
                .unwrap()
                .world
                .transform_point3(glam::Vec3::ZERO)
                .truncate();
            menu::take();
            h.run(pad::BACK, 1);
            h.run(0, 1);
            let mut sounds = menu::take();
            let before = h.settings;
            assert_eq!(h.screens.top(), Some(name));
            if inject {
                let mut env =
                    Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
                assert!(h.screens.click(point, &mut env).is_none());
                assert!(!h.screens.focus_button_at(point), "pointer fallback must not move native focus during exit");
                assert!(!h.screens.release_at(point, crate::frontend::ids::MOUSE_LEFT_RELEASED));
                h.screens.post_to_top(0, blackbox_feng::ids::PAD_RIGHT);
                h.screens.post_to_top(0, blackbox_feng::ids::PAD_START);
            }
            h.run(0, 2);
            sounds.extend(menu::take());
            assert_eq!(h.settings, before);
            assert_eq!(h.screens.top(), Some(name), "the accepted exit is still pending");
            assert!(!h.said(&Command::Resume), "an exit cannot be replaced by a raw Start");
            h.wait(1.5);
            sounds.extend(menu::take());
            assert_eq!(h.screens.top(), Some(if pause { screen::PAUSE_MENU } else { screen::MAIN_MENU_SUB }));
            assert_eq!(h.commands.iter().filter(|command| **command == Command::SaveSettings).count(), 1);
            if inject {
                assert_eq!(
                    &sounds,
                    control.as_ref().unwrap(),
                    "rejected pointer/raw input must preserve the complete exit sound sequence"
                );
            } else {
                control = Some(sounds);
            }
        }
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn batch12_pause_exit_rejects_spammed_back_start_and_direction_feedback() {
    fn targets(h: &Harness) -> (glam::Vec2, u32, glam::Vec2) {
        let tree = h.screens.trees().pop().unwrap();
        let focused = h.screens.focused_button().expect("native pause button");
        let button = tree.nodes.iter().find(|node| node.index == focused.index && node.visible).unwrap();
        let arrow = tree.nodes.iter().find(|node| node.name_hash == 0x4449_69fe && node.visible).unwrap();
        let point = |node: &blackbox_feng::UiNode| node.world.transform_point3(glam::Vec3::ZERO).truncate();
        (point(button), button.name_hash, point(arrow))
    }
    // Prove these native targets accept the same API routes before an exit. Native pad disabling does not guard
    // raw posts or the runtime's pointer hit tests; the host's pending-state policy must reject them.
    let mut probe = Harness::open(screen::PAUSE_MENU, Args { pause: true, ..Args::default() }).unwrap();
    probe.wait(1.0);
    let (point, button, arrow) = targets(&probe);
    assert!(probe.screens.focus_button_at(point));
    menu::take();
    assert!(probe.screens.release_at(arrow, crate::frontend::ids::MOUSE_LEFT_RELEASED));
    probe.run(0, 2);
    assert!(menu::take().contains(&2), "the native arrow release must actually request directional feedback");
    probe.screens.post_to_top(button, blackbox_feng::ids::BUTTON_PRESSED);
    probe.run(0, 2);
    assert!(menu::take().contains(&4), "the native button raw post must actually request acceptance feedback");

    for action in [pad::BACK, pad::START, pad::ACCEPT] {
        let mut control = None;
        for spam in [false, true] {
            let mut h =
                Harness::open(screen::PAUSE_MENU, Args { pause: true, ..Args::default() }).expect("installed menus");
            h.wait(1.0);
            let (point, button, arrow) = targets(&h);
            menu::take();
            h.run(action, 1);
            let mut sounds = menu::take();
            let fallback = spam.then(|| {
                let focused = h.screens.focus_button_at(point);
                let released = h.screens.release_at(arrow, crate::frontend::ids::MOUSE_LEFT_RELEASED);
                h.screens.post_to_top(button, blackbox_feng::ids::BUTTON_PRESSED);
                (focused, released)
            });
            // Before the authored exit completes, press and release several buttons on separate frames.
            for press in [pad::BACK, pad::START, pad::RIGHT, pad::ACCEPT] {
                h.run(0, 1);
                h.run(if spam { press } else { 0 }, 1);
                sounds.extend(menu::take());
            }
            assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU), "the accepted exit is still pending");
            assert!(!h.said(&Command::Resume));
            h.run(0, 90);
            sounds.extend(menu::take());
            assert_eq!(h.commands.iter().filter(|command| **command == Command::Resume).count(), 1);
            if spam {
                assert_eq!(
                    &sounds,
                    control.as_ref().unwrap(),
                    "spam must preserve the complete accepted-exit sound sequence"
                );
                assert_eq!(fallback, Some((false, false)), "pending exit must reject both native pointer fallbacks");
            } else {
                control = Some(sounds);
            }
        }
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn batch12_safe_house_confirmation_warns_for_pursuit_and_cancel_retains_pause() {
    use crate::frontend::icon_items::{Id, pause_icons};
    use crate::frontend::logic::{PauseCtx, QuitTo};
    for pursuit in [false, true] {
        let ctx = PauseCtx { safe_house: Some(true), pursuit, quit: QuitTo::Career, ..Default::default() };
        let mut h = Harness::open(screen::PAUSE_MENU, Args { pause: true, pause_ctx: ctx, ..Default::default() })
            .expect("installed menus");
        h.wait(1.0);
        let target = pause_icons(ctx).iter().position(|icon| icon.id == Id::JumpToSafeHouse).unwrap();
        for _ in 0..target {
            h.press(pad::RIGHT);
        }
        h.press(pad::ACCEPT);
        h.wait(1.0);
        assert_eq!(h.screens.top(), Some(screen::DIALOG));
        let texts: Vec<_> = h
            .screens
            .trees()
            .into_iter()
            .flat_map(|tree| tree.nodes.into_iter().filter_map(|node| node.text))
            .collect();
        assert!(texts.iter().any(|text| text.contains("jump to the Safe House")), "{texts:?}");
        assert_eq!(texts.iter().any(|text| text.contains("All pursuit progress will be lost")), pursuit);
        h.press(pad::BACK);
        h.wait(0.5);
        assert_eq!(h.screens.top(), Some(screen::PAUSE_MENU));
        assert!(!h.said(&Command::JumpToSafeHouse));
        h.press(pad::ACCEPT);
        h.wait(0.5);
        h.press(pad::LEFT);
        h.press(pad::ACCEPT);
        h.wait(0.5);
        assert_eq!(h.commands.iter().filter(|command| **command == Command::JumpToSafeHouse).count(), 1);
    }
}
