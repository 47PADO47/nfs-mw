//! Installed package-to-audio integration, including native pointer paths.
use super::*;
use crate::audio::menu;

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn installed_menu_gestures_queue_one_accept_and_navigation_and_back() {
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
    h.wait(1.0);
    menu::take();
    h.press(pad::RIGHT);
    assert_eq!(menu::take().iter().filter(|&&id| id == 2).count(), 1);
    h.press(pad::ACCEPT);
    h.wait(1.5);
    let sounds = menu::take();
    assert_eq!(sounds.iter().filter(|&&id| id == 4).count(), 1, "{sounds:?}");
    h.press(pad::BACK);
    h.wait(1.5);
    assert_eq!(menu::take().iter().filter(|&&id| id == 5).count(), 1);
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn installed_dialog_click_queues_accept_and_a_miss_is_silent() {
    let mut h = gps_dialog(Command::SetWaypoint([1.0, 2.0])).expect("installed menus");
    menu::take();
    assert!(h.click(glam::Vec2::new(-400.0, -300.0)));
    assert!(menu::take().is_empty());
    let no = centre_of(&h, crate::frontend::confirm::NO_BUTTON).expect("No answer");
    assert!(h.click(no));
    h.wait(1.5);
    let sounds = menu::take();
    assert_eq!(sounds.iter().filter(|&&id| id == 4).count(), 1, "{sounds:?}");
    assert_eq!(h.screens.top(), Some(screen::OPTIONS));
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn native_setting_pointer_edits_sound_only_when_the_value_changes() {
    let mut h =
        Harness::open(screen::OPTIONS, Args { category: Category::Audio, ..Args::default() }).expect("installed menus");
    h.wait(1.5);
    let (data, _) = h.node("OPTION_DATA_1");
    menu::take();
    assert!(h.click(data));
    assert!(menu::take().is_empty(), "selection without an edit is silent");
    let (right, visible) = h.node("RIGHT_ARROW_1");
    assert!(visible);
    h.settings.master_volume = crate::settings::Percent(70);
    assert!(h.click(right));
    assert_ne!(h.settings.master_volume, crate::settings::Percent(70));
    assert_eq!(menu::take(), [2]);
    h.settings.master_volume = crate::settings::Percent(100);
    assert!(h.click(right));
    assert_eq!(h.settings.master_volume, crate::settings::Percent(100));
    assert!(menu::take().is_empty(), "an edit already at its limit is silent");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn remastered_disabled_icon_and_pointer_misses_do_not_sound_like_acceptance() {
    use crate::ui::theme::remastered::menu::layout;
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
    h.screens.clear();
    h.settings.ui_style = crate::settings::UiStyle::Remastered;
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open(screen::MAIN_MENU, Args::default(), true, &mut env);
    h.wait(1.5);
    let list = h
        .screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Xbox)
        .pop()
        .unwrap()
        .list
        .expect("remastered entries");
    let row = list.entries.iter().position(|entry| !entry.enabled).expect("disabled My Cars");
    let point = glam::Vec2::new(layout::left_edge(list.aspect) + 100.0, layout::row_y(row));
    menu::take();
    assert!(h.click(point));
    assert!(menu::take().is_empty(), "disabled entries do not confirm an action");
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU));
    h.click(glam::Vec2::new(-1000.0, -1000.0));
    assert!(menu::take().is_empty(), "missing all rows is silent");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn remastered_setting_click_at_its_limit_is_silent() {
    use crate::ui::theme::remastered::menu::layout;
    let args = Args { category: Category::Audio, ..Args::default() };
    let mut h = Harness::open(screen::OPTIONS, args).expect("installed menus");
    h.screens.clear();
    h.settings.ui_style = crate::settings::UiStyle::Remastered;
    h.settings.master_volume = crate::settings::Percent(100);
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open(screen::OPTIONS, args, true, &mut env);
    h.wait(1.5);
    menu::take();
    let point = glam::Vec2::new(layout::left_edge(crate::ui::theme::DEFAULT_ASPECT) + 100.0, layout::row_y(0));
    assert!(h.click(point));
    assert_eq!(h.settings.master_volume, crate::settings::Percent(100));
    assert!(menu::take().is_empty(), "clamped settings do not play acceptance feedback");
    h.settings.master_volume = crate::settings::Percent(70);
    assert!(h.click(point));
    assert_ne!(h.settings.master_volume, crate::settings::Percent(70));
    assert_eq!(menu::take(), [2], "a successful setting change has one directional effect");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn native_icon_pointer_activation_has_one_accept_across_the_transition() {
    use crate::frontend::icon_items::{Id, icons};
    use crate::frontend::icon_menu::Kind;
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
    h.wait(1.5);
    let target = icons(Kind::Main).into_iter().find(|icon| icon.id == Id::QuickRace).unwrap();
    let tree = h.screens.trees().pop().unwrap();
    let point = tree
        .nodes
        .iter()
        .find(|n| {
            n.visible
                && matches!(n.kind,
        blackbox_feng::NodeKind::Image { texture, .. } if texture == target.texture)
        })
        .unwrap()
        .world
        .transform_point3(glam::Vec3::ZERO)
        .truncate();
    menu::take();
    assert!(h.click(point));
    h.wait(2.0);
    assert_eq!(h.screens.top(), Some(screen::MAIN_MENU_SUB));
    let sounds = menu::take();
    assert_eq!(sounds.iter().filter(|&&id| id == 4).count(), 1, "{sounds:?}");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn remastered_controls_pointer_activation_has_one_accept_after_updates() {
    use crate::ui::theme::remastered::menu::layout;
    let mut h = Harness::open(screen::CONTROLS, Args::default()).expect("installed menus");
    h.screens.clear();
    h.settings.ui_style = crate::settings::UiStyle::Remastered;
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open(screen::CONTROLS, Args::default(), true, &mut env);
    h.wait(1.5);
    menu::take();
    let point = glam::Vec2::new(layout::left_edge(crate::ui::theme::DEFAULT_ASPECT) + 100.0, layout::row_y(0));
    assert!(h.click(point));
    h.wait(1.5);
    assert_eq!(h.commands.iter().filter(|command| matches!(command, Command::Remap(..))).count(), 1);
    let sounds = menu::take();
    assert_eq!(sounds.iter().filter(|&&id| id == 4).count(), 1, "{sounds:?}");
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn empty_name_pointer_activation_stays_silent_across_updates() {
    use crate::frontend::name_entry::{NameEntry, NameFor, TextKey};
    use crate::ui::theme::remastered::menu::layout;
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
    h.settings.ui_style = crate::settings::UiStyle::Remastered;
    {
        let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
        h.screens.open_with(screen::DIALOG, Box::new(NameEntry::new(NameFor::Create)), false, &mut env);
        for _ in 0..64 {
            h.screens.type_key(TextKey::Backspace, &mut env);
        }
    }
    h.wait(1.5);
    let list = h
        .screens
        .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Xbox)
        .pop()
        .unwrap()
        .list
        .expect("name prompt");
    let point = (-800..800)
        .map(|y| glam::Vec2::new(0.0, y as f32 * 0.25))
        .find(|&point| layout::dialog_row_at(point, &list) == Some(0))
        .expect("name acceptance row");
    menu::take();
    assert!(h.click(point));
    h.wait(1.5);
    assert_eq!(h.screens.top(), Some(screen::DIALOG));
    assert!(menu::take().is_empty());
}
