//! The selection over an option page that has section titles (docs/specs/menu-sections.md, section 3), on the installed
//! option package (the Original look) and on the Remastered and Custom lists (skipped without `NFSMW_GAME_DIR`): the arrows,
//! the wrap, the page jumps and the pointer pass over a title, and a press on one does nothing.

use super::*;
use crate::frontend::options::{self, Setting};
use crate::settings::UiStyle;
use crate::ui::theme::MenuList;
use crate::ui::theme::remastered::menu::layout as remastered_layout;

fn open(category: Category, style: UiStyle) -> Option<Harness> {
    let mut h = Harness::open(screen::OPTIONS, Args { category, ..Args::default() })?;
    h.settings.ui_style = style;
    h.wait(1.5);
    Some(h)
}

/// The slots of the package page whose right arrow shows: only the selected row's does.
fn selected_slots(h: &Harness) -> Vec<usize> {
    (1..=10)
        .filter(|n| {
            let hash = blackbox_feng::fe_hash_upper(&format!("RIGHT_ARROW_{n}"));
            h.screens
                .trees()
                .pop()
                .is_some_and(|tree| tree.nodes.iter().any(|node| node.name_hash == hash && node.visible))
        })
        .collect()
}

/// The list the top screen publishes for a list look.
fn list(h: &Harness) -> MenuList {
    let presented = h.screens.presented(&crate::input::Bindings::default(), crate::input::InputDevice::Xbox);
    presented.into_iter().rev().find_map(|page| page.list).expect("a list look publishes the page")
}

fn page_rows() -> Vec<options::Row> {
    options::page(Category::Gameplay, &test_caps::native(), Vec::new())
}

fn index_of(setting: Setting) -> usize {
    options::rows(Category::Gameplay, &test_caps::native()).iter().position(|row| row.setting == setting).unwrap()
}

#[test]
fn down_and_up_pass_over_the_titles_and_the_wrap_lands_on_a_row() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Original) else { return };
    // Down to Turn Signals, the first row under Experimental: one press for each row before it, none for the title.
    for _ in 0..index_of(Setting::TurnSignals) {
        h.press(pad::DOWN);
        h.wait(0.6);
        assert_eq!(selected_slots(&h).len(), 1, "exactly one row is selected after every press, never a title");
    }
    h.press(pad::RIGHT);
    assert!(h.changed.turn_signals.is_some(), "the press after the title's skip changes Turn Signals");
    // Up from it passes the title again, onto the last untitled row, then Camera above it.
    h.press(pad::UP);
    h.press(pad::UP);
    h.press(pad::RIGHT);
    assert!(h.changed.camera_mode.is_some(), "two presses up from Turn Signals are Camera, over the title");
    // Up from the first row wraps to the last row of the page, which is a row and not a title.
    let mut h = open(Category::Gameplay, UiStyle::Original).unwrap();
    h.press(pad::UP);
    h.press(pad::RIGHT);
    assert!(h.changed.clutch_fingers.is_some(), "the last row of the page, under RIDE 6");
    h.press(pad::DOWN);
    h.press(pad::RIGHT);
    assert!(h.changed.hud_layout.is_some(), "and down from it wraps to the first row");
}

#[test]
fn every_press_of_down_selects_a_row_over_the_whole_page_and_the_wheel_does_the_same() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Original) else { return };
    let rows = page_rows().len();
    for press in 0..rows + 2 {
        h.press(pad::DOWN);
        h.wait(0.6);
        assert_eq!(selected_slots(&h).len(), 1, "press {press}: one selected row");
    }
    h.scroll(5);
    h.wait(0.3);
    assert_eq!(selected_slots(&h).len(), 1, "the wheel up lands on rows too");
    h.scroll(-9);
    h.wait(0.3);
    assert_eq!(selected_slots(&h).len(), 1, "and the wheel down");
}

#[test]
fn a_click_on_a_title_does_nothing_and_the_row_selected_stays() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Original) else { return };
    // Scroll until a title is in the window, and find the slot that shows it.
    let first_title = page_rows().iter().position(|row| row.setting == Setting::Heading).unwrap();
    for _ in 0..first_title {
        h.press(pad::DOWN);
    }
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    let title =
        tree.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Experimental")).expect("the title shows");
    let point = title.world.transform_point3(glam::Vec3::ZERO).truncate();
    let (before, selected) = (h.settings, selected_slots(&h));
    assert!(h.click(point), "a click on the title is taken by the page");
    h.wait(0.6);
    assert_eq!(h.settings, before, "and changes nothing");
    assert_eq!(selected_slots(&h), selected, "and does not move the selection");
}

#[test]
fn a_click_on_a_row_below_the_first_selects_that_row() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Original) else { return };
    let (point, _) = h.node("OPTION_DATA_5");
    assert!(h.click(point));
    h.wait(0.6);
    assert_eq!(selected_slots(&h), [5], "the fifth row is selected, not the first");
}

#[test]
fn a_title_has_the_colour_the_selected_row_title_has() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Original) else { return };
    let first_title = page_rows().iter().position(|row| row.setting == Setting::Heading).unwrap();
    for _ in 0..first_title {
        h.press(pad::DOWN);
        h.wait(0.3);
    }
    h.wait(1.0);
    let tree = h.screens.trees().pop().unwrap();
    let selected = selected_slots(&h)[0];
    let name = |n: usize| {
        let hash = blackbox_feng::fe_hash_upper(&format!("OPTION_NAME_{n}"));
        tree.nodes.iter().find(|node| node.name_hash == hash).unwrap().clone()
    };
    let title = tree.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Experimental")).unwrap();
    let rgb = |colour: [u8; 4]| [colour[0], colour[1], colour[2]];
    let other = name(if selected == 1 { 2 } else { 1 });
    assert_eq!(rgb(title.colour), rgb(name(selected).colour), "the colour the selected row's title has");
    assert_ne!(rgb(title.colour), rgb(other.colour), "and not the colour of an ordinary title");
}

#[test]
fn a_title_in_a_slot_that_held_a_dimmed_row_is_not_dimmed() {
    // Ray Tracing is dimmed on a renderer without it, and the page scrolls so the Experimental title takes its place.
    let Some(mut h) = open(Category::Video, UiStyle::Original) else { return };
    h.press(pad::UP);
    h.wait(1.5);
    let tree = h.screens.trees().pop().unwrap();
    let title = tree.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Experimental")).expect("it shows");
    let dimmed = tree.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Ray Tracing")).expect("it shows");
    assert!(dimmed.world_colour[3] < title.world_colour[3], "Ray Tracing is dimmed and the title is not");
    assert!(title.world_colour[3] >= 250, "the title is at full strength: {}", title.world_colour[3]);
}

#[test]
fn a_title_is_centred_across_the_package_rows() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Original) else { return };
    let first_title = page_rows().iter().position(|row| row.setting == Setting::Heading).unwrap();
    for _ in 0..first_title {
        h.press(pad::DOWN);
    }
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    let title =
        tree.nodes.iter().find(|n| n.visible && n.text.as_deref() == Some("Experimental")).expect("the title shows");
    let x = title.world.transform_point3(glam::Vec3::ZERO).x;
    let columns: Vec<f32> = ["OPTION_NAME_1", "OPTION_DATA_1"]
        .iter()
        .map(|name| {
            let hash = blackbox_feng::fe_hash_upper(name);
            tree.nodes.iter().find(|n| n.name_hash == hash).unwrap().world.transform_point3(glam::Vec3::ZERO).x
        })
        .collect();
    assert!(x > columns[0] + 20.0, "to the right of where the right aligned titles end ({x} against {})", columns[0]);
}

fn hover(h: &mut Harness, index: usize, first: usize) {
    let left = remastered_layout::left_edge(crate::ui::theme::DEFAULT_ASPECT);
    let point = glam::Vec2::new(left + 100.0, remastered_layout::row_y(index - first));
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.hover_row(None, &mut env);
    h.screens.hover_row(Some(point), &mut env);
}

#[test]
fn the_remastered_list_marks_the_titles_and_never_selects_them() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Remastered) else { return };
    let list_now = list(&h);
    let page = page_rows();
    assert_eq!(list_now.entries.len(), page.len(), "a title is an entry of the list");
    for (entry, row) in list_now.entries.iter().zip(&page) {
        assert_eq!(entry.flags.stops_scroll(), !row.setting.is_heading());
        assert_eq!(entry.flags.centered, row.setting.is_heading(), "{}", entry.label);
    }
    assert_eq!(list_now.selected, 0);
    for _ in 0..page.len() + 3 {
        h.press(pad::DOWN);
        let now = list(&h);
        assert!(now.entries[now.selected].flags.highlightable, "{} is selected", now.entries[now.selected].label);
    }
}

#[test]
fn the_pointer_does_not_select_or_press_a_title_in_the_remastered_list() {
    let Some(mut h) = open(Category::Gameplay, UiStyle::Remastered) else { return };
    let page = page_rows();
    let title = page.iter().position(|row| row.setting == Setting::Heading).unwrap();
    // Scroll the window so the title shows, then point at it.
    for _ in 0..title {
        h.press(pad::DOWN);
    }
    let now = list(&h);
    let (selected, first) = (now.selected, now.first);
    assert!(title >= first && title < first + 8, "the title is in the window");
    hover(&mut h, title, first);
    assert_eq!(list(&h).selected, selected, "hover leaves the selection where it was");
    let left = remastered_layout::left_edge(crate::ui::theme::DEFAULT_ASPECT);
    let point = glam::Vec2::new(left + 100.0, remastered_layout::row_y(title - first));
    let before = h.settings;
    h.click(point);
    assert_eq!(h.settings, before, "a press on the title changes nothing");
    assert_eq!(list(&h).selected, selected, "and selects nothing");
    // The row under the title is selected by hover as any row is.
    let under = title + 1;
    if under < first + 8 {
        hover(&mut h, under, first);
        assert_eq!(list(&h).selected, under);
    }
}

#[test]
fn the_custom_list_marks_the_titles_the_same_way() {
    let Some(h) = open(Category::Video, UiStyle::Custom) else { return };
    let list_now = list(&h);
    let titles: Vec<&str> =
        list_now.entries.iter().filter(|entry| entry.flags.centered).map(|entry| entry.label.as_str()).collect();
    assert_eq!(titles, ["Experimental"]);
    assert!(list_now.entries.iter().all(|entry| entry.flags.centered == !entry.flags.stops_scroll()));
}
