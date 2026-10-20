//! Tests for the Remastered vertical menu: what it builds from a list, with no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, NodeKind, Rect, Theme, UiNode};

use super::layout::{left_edge, row_y};
use crate::input::Action;
use crate::ui::input_icons::{ATLAS, Glyph};
use crate::ui::prompts::Prompt;
use crate::ui::theme::custom::{DriveFeedCell, palette};
use crate::ui::theme::remastered::{RemasteredTheme, palette_skin, skin};
use crate::ui::theme::{ElementId, MenuEntry, MenuHint, MenuList, MenuListCell, MenuStyle};

const ASPECT: f32 = 16.0 / 9.0;

fn entry(label: &str, enabled: bool) -> MenuEntry {
    MenuEntry::plain(label, enabled)
}

fn list(style: MenuStyle, selected: usize) -> MenuList {
    MenuList {
        title: "Main menu".into(),
        message: String::new(),
        entries: vec![entry("Career", true), entry("Challenge Series", false), entry("Quick Race", true)],
        selected,
        first: 0,
        hints: vec![
            MenuHint { keys: vec![Prompt::Key("Enter".into())], label: "Accept", action: Some(Action::MenuAccept) },
            MenuHint { keys: vec![Prompt::Icon(Glyph::B)], label: "Back", action: Some(Action::MenuBack) },
        ],
        style,
        nudge: None,
        scroll: 0.0,
        aspect: ASPECT,
    }
}

fn built(slot: ElementId, cell: &MenuListCell) -> Vec<UiNode> {
    let skin = palette::remastered().skin().merged(skin()).merged(palette_skin(&palette::remastered()));
    let theme = RemasteredTheme::new(DriveFeedCell::new(), cell.clone(), Default::default());
    let mut menu = theme.element(slot.slot(), &BuildCx::new(&skin)).expect("Remastered draws this menu");
    menu.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    menu.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

fn text_at<'a>(nodes: &'a [UiNode], text: &str) -> &'a UiNode {
    nodes.iter().find(|n| n.text.as_deref() == Some(text)).unwrap_or_else(|| panic!("no {text:?} drawn"))
}

fn images_with(nodes: &[UiNode], wanted: u32) -> usize {
    nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == wanted)).count()
}

#[test]
fn the_icon_menus_and_the_settings_pages_are_vertical_and_the_rest_fall_back() {
    let theme = RemasteredTheme::default();
    let skin = skin();
    let cx = BuildCx::new(&skin);
    for slot in [
        ElementId::MainMenu,
        ElementId::Categories,
        ElementId::PauseMenu,
        ElementId::SafeHouseMenu,
        ElementId::OptionTable,
    ] {
        assert!(theme.element(slot.slot(), &cx).is_some(), "{} is vertical", slot.name());
    }
    for slot in [ElementId::TrackSelect, ElementId::Blacklist, ElementId::CarLot] {
        assert!(theme.element(slot.slot(), &cx).is_none(), "{} stays Original", slot.name());
    }
}

#[test]
fn a_slot_with_no_list_draws_nothing() {
    assert!(built(ElementId::MainMenu, &MenuListCell::new()).is_empty());
}

#[test]
fn the_title_and_the_entries_are_capitals_in_one_column_on_the_reference_rows() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
    let nodes = built(ElementId::MainMenu, &cell);
    assert!(texts(&nodes).contains(&"MAIN MENU"));
    for (i, label) in ["CAREER", "CHALLENGE SERIES", "QUICK RACE"].iter().enumerate() {
        let node = text_at(&nodes, label);
        assert!((node.world.w_axis.y - row_y(i)).abs() < 1e-3, "{label} sits on its row");
        assert!(node.world.w_axis.x > left_edge(ASPECT), "{label} is inside the screen");
    }
    let x = |label: &str| text_at(&nodes, label).world.w_axis.x;
    assert_eq!(x("CAREER"), x("QUICK RACE"), "one left edge for every entry");
}

#[test]
fn only_the_selected_entry_has_the_gold_bar_and_a_disabled_one_is_dimmed() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 2));
    let nodes = built(ElementId::MainMenu, &cell);
    let on_row = |i: usize| {
        nodes
            .iter()
            .filter(|n| {
                matches!(n.kind, NodeKind::Image { texture: 0, .. }) && (n.world.w_axis.y - row_y(i)).abs() < 0.5
            })
            .count()
    };
    assert!(on_row(2) > 20, "the bar's strips are on the selected row");
    assert_eq!(on_row(0), 0, "and no other row has them");
    let career = text_at(&nodes, "CAREER").colour;
    assert!(text_at(&nodes, "CHALLENGE SERIES").colour[3] < career[3], "a disabled entry is fainter");
    assert!(text_at(&nodes, "QUICK RACE").colour[0] >= career[0], "the selected entry is the brightest");
}

#[test]
fn the_logo_and_the_tagline_depend_on_the_menu() {
    let logo = blackbox_feng::resource_handle("MW_Logo.tga");
    let cell = MenuListCell::new();
    for (style, has_logo, tagline) in
        [(MenuStyle::Main, true, true), (MenuStyle::Sub, true, false), (MenuStyle::Pause, false, false)]
    {
        cell.set(ElementId::MainMenu.slot(), list(style, 0));
        let nodes = built(ElementId::MainMenu, &cell);
        assert_eq!(images_with(&nodes, logo) == 1, has_logo, "{style:?} logo");
        assert_eq!(texts(&nodes).contains(&"RACE YOUR WAY"), tagline, "{style:?} tagline");
    }
}

#[test]
fn the_footer_draws_each_hint_with_its_key_and_label() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
    let nodes = built(ElementId::MainMenu, &cell);
    let all = texts(&nodes);
    assert!(all.contains(&"ENTER") && all.contains(&"Accept") && all.contains(&"Back"), "{all:?}");
    assert_eq!(images_with(&nodes, ATLAS), 1, "the pad glyph of the Back hint");
    let mut bare = list(MenuStyle::Main, 0);
    bare.hints.clear();
    cell.set(ElementId::MainMenu.slot(), bare);
    assert!(!texts(&built(ElementId::MainMenu, &cell)).contains(&"Accept"), "no hints, no footer");
}

#[test]
fn a_menu_is_placed_from_the_left_edge_of_the_screen() {
    let cell = MenuListCell::new();
    let mut menu = list(MenuStyle::Pause, 0);
    cell.set(ElementId::PauseMenu.slot(), menu.clone());
    let wide = text_at(&built(ElementId::PauseMenu, &cell), "CAREER").world.w_axis.x;
    menu.aspect = 4.0 / 3.0;
    cell.set(ElementId::PauseMenu.slot(), menu);
    let narrow = text_at(&built(ElementId::PauseMenu, &cell), "CAREER").world.w_axis.x;
    assert!((wide - narrow - (left_edge(ASPECT) - left_edge(4.0 / 3.0))).abs() < 1e-3);
}

#[test]
fn a_long_list_shows_a_window_of_it_and_no_scroll_bar() {
    let cell = MenuListCell::new();
    let mut long = list(MenuStyle::Sub, 5);
    long.entries = (0..12).map(|i| entry(&format!("Item {i}"), true)).collect();
    long.first = 3;
    cell.set(ElementId::Categories.slot(), long);
    let nodes = built(ElementId::Categories, &cell);
    let all = texts(&nodes);
    assert!(all.contains(&"ITEM 3") && all.contains(&"ITEM 10"), "eight entries from the window's start: {all:?}");
    assert!(!all.contains(&"ITEM 2") && !all.contains(&"ITEM 11"), "none outside the window: {all:?}");
    assert!(
        (text_at(&nodes, "ITEM 3").world.w_axis.y - row_y(0)).abs() < 1e-3,
        "the window's first entry is the first row"
    );
    let thin = nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture: 0, .. }) && n.world.x_axis.x < 3.0);
    assert_eq!(thin.filter(|n| n.world.y_axis.y > 100.0).count(), 0, "no tall thin bar beside the list");
}

#[test]
fn the_title_is_bigger_and_bold_in_the_accent_colour() {
    let cell = MenuListCell::new();
    cell.set(ElementId::MainMenu.slot(), list(MenuStyle::Main, 0));
    let nodes = built(ElementId::MainMenu, &cell);
    let title = text_at(&nodes, "MAIN MENU");
    let entry = text_at(&nodes, "CAREER");
    assert!(title.world.y_axis.y > entry.world.y_axis.y, "the title is set larger than the entries");
    let NodeKind::Text { font, .. } = title.kind else { panic!("the title is text") };
    assert_eq!(font, crate::ui::raster_text::RASTER_MENU_TITLE_FONT_KEY, "in the bold face");
    let accent = palette::remastered().secondary;
    assert_eq!(title.colour, accent, "and in the palette's accent");
}

fn settings(selected: usize) -> MenuList {
    use crate::ui::theme::MenuValue;
    let row = |label: &str, value: MenuValue, enabled: bool| MenuEntry { label: label.into(), enabled, value };
    MenuList {
        entries: vec![
            row("Window Mode", MenuValue::Choice { text: "Borderless".into(), arrows: true }, true),
            row("Master Volume", MenuValue::Slider(75), true),
            row("Advanced Graphics", MenuValue::Choice { text: "Open".into(), arrows: false }, true),
            row("Ray Tracing", MenuValue::Choice { text: "Unavailable".into(), arrows: true }, false),
        ],
        ..list(MenuStyle::Sub, selected)
    }
}

fn arrows(nodes: &[UiNode]) -> Vec<&UiNode> {
    let arrow = blackbox_feng::resource_handle("ARROW_SKINNIER.tga");
    nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture, .. } if texture == arrow)).collect()
}

#[test]
fn a_settings_row_shows_its_value_in_a_column_and_only_the_selected_row_has_chevrons() {
    let cell = MenuListCell::new();
    cell.set(ElementId::OptionTable.slot(), settings(0));
    let nodes = built(ElementId::OptionTable, &cell);
    let all = texts(&nodes);
    assert!(all.contains(&"WINDOW MODE") && all.contains(&"BORDERLESS") && all.contains(&"75"), "{all:?}");
    assert_eq!(arrows(&nodes).len(), 2, "one pair, on the selected row");
    let (label, value) = (text_at(&nodes, "WINDOW MODE"), text_at(&nodes, "BORDERLESS"));
    assert!(value.world.w_axis.x > label.world.w_axis.x + 150.0, "the value is in a column to the right");
}

#[test]
fn a_row_that_acts_or_is_disabled_has_no_chevrons_and_a_disabled_value_is_dim() {
    let cell = MenuListCell::new();
    cell.set(ElementId::OptionTable.slot(), settings(2));
    assert!(arrows(&built(ElementId::OptionTable, &cell)).is_empty(), "a row that only opens something");
    cell.set(ElementId::OptionTable.slot(), settings(3));
    let nodes = built(ElementId::OptionTable, &cell);
    assert!(arrows(&nodes).is_empty(), "a disabled row");
    assert!(text_at(&nodes, "UNAVAILABLE").colour[3] < 150, "its value is faint");
}

#[test]
fn a_slider_row_draws_a_bar_and_a_press_nudges_the_chevron_it_pressed() {
    let cell = MenuListCell::new();
    cell.set(ElementId::OptionTable.slot(), settings(1));
    let rest = built(ElementId::OptionTable, &cell);
    assert_eq!(arrows(&rest).len(), 2);
    let mut pressed = settings(1);
    pressed.nudge = Some(crate::ui::theme::MenuNudge { row: 1, forward: true, age: 0.0 });
    cell.set(ElementId::OptionTable.slot(), pressed);
    let nudged = built(ElementId::OptionTable, &cell);
    let xs = |nodes: &[UiNode]| arrows(nodes).iter().map(|n| n.world.w_axis.x).collect::<Vec<_>>();
    let (before, after) = (xs(&rest), xs(&nudged));
    assert_eq!(before[0], after[0], "the left chevron stays");
    assert!(after[1] > before[1] + 3.0, "the right one is out");
}

#[test]
fn a_settings_bar_runs_wider_than_an_icon_menus_and_a_long_label_shrinks_to_fit() {
    let cell = MenuListCell::new();
    let right_edge = |list: MenuList| {
        cell.set(ElementId::OptionTable.slot(), list);
        built(ElementId::OptionTable, &cell)
            .iter()
            .filter(|n| {
                matches!(n.kind, NodeKind::Image { texture: 0, .. }) && n.colour[3] > 100 && n.world.y_axis.y > 20.0
            })
            .map(|n| n.world.w_axis.x + n.world.x_axis.x / 2.0)
            .fold(f32::MIN, f32::max)
    };
    let narrow = right_edge(list(MenuStyle::Sub, 0));
    let wide = right_edge(settings(0));
    assert!(wide > narrow + 80.0, "{wide} against {narrow}");

    let mut long = settings(0);
    long.entries[0].label = "An Extremely Long Setting Name That Cannot Fit".into();
    cell.set(ElementId::OptionTable.slot(), long);
    let nodes = built(ElementId::OptionTable, &cell);
    let shrunk = text_at(&nodes, "AN EXTREMELY LONG SETTING NAME THAT CANNOT FIT").world.y_axis.y;
    assert!(shrunk < text_at(&nodes, "MASTER VOLUME").world.y_axis.y, "set smaller than the others");
}

#[test]
fn the_in_game_menu_dims_the_whole_screen_and_the_main_menu_does_not() {
    let cell = MenuListCell::new();
    let scrim = |style| {
        cell.set(ElementId::PauseMenu.slot(), list(style, 0));
        built(ElementId::PauseMenu, &cell)
            .iter()
            .any(|n| n.world.x_axis.x > 800.0 && n.world.y_axis.y > 480.0 && n.colour[3] > 100)
    };
    assert!(scrim(MenuStyle::Pause), "the frozen world is dimmed");
    assert!(!scrim(MenuStyle::Main), "the title screen's menu is not");
}

fn dialog(selected: usize, answers: &[&str]) -> MenuList {
    MenuList {
        title: "Save/Load".into(),
        message: "Are you sure you want to delete this profile? This cannot be undone.".into(),
        entries: answers.iter().map(|a| entry(a, true)).collect(),
        selected,
        ..list(MenuStyle::Dialog, selected)
    }
}

#[test]
fn a_dialog_is_a_scrim_and_a_panel_with_the_title_the_wrapped_message_and_the_answers_one_under_the_other() {
    let cell = MenuListCell::new();
    cell.set(ElementId::Dialog.slot(), dialog(1, &["Yes", "No"]));
    let nodes = built(ElementId::Dialog, &cell);
    let all = texts(&nodes);
    assert!(all.contains(&"SAVE/LOAD") && all.contains(&"YES") && all.contains(&"NO"), "{all:?}");
    assert!(all.iter().filter(|t| t.contains("profile") || t.contains("undone")).count() >= 1, "the message is drawn");
    assert!(nodes.iter().any(|n| n.world.x_axis.x > 800.0 && n.world.y_axis.y > 480.0), "the scrim covers the screen");
    let (yes, no) = (text_at(&nodes, "YES"), text_at(&nodes, "NO"));
    assert!(no.world.w_axis.y > yes.world.w_axis.y + 20.0, "No is under Yes");
    assert_eq!(yes.world.w_axis.x, no.world.w_axis.x, "in one column");
    assert!(yes.colour[0] <= no.colour[0] && yes.colour != no.colour, "the selected answer is the brighter");
}

#[test]
fn a_notice_has_one_answer_and_a_longer_message_makes_a_taller_panel() {
    let cell = MenuListCell::new();
    cell.set(ElementId::Dialog.slot(), dialog(0, &["Continue"]));
    let nodes = built(ElementId::Dialog, &cell);
    assert!(texts(&nodes).contains(&"CONTINUE"));
    let height_of = |message: &str| {
        let mut list = dialog(0, &["Continue"]);
        list.message = message.into();
        cell.set(ElementId::Dialog.slot(), list);
        let ys: Vec<f32> =
            built(ElementId::Dialog, &cell).iter().filter(|n| n.text.is_some()).map(|n| n.world.w_axis.y).collect();
        ys.iter().cloned().fold(f32::MIN, f32::max) - ys.iter().cloned().fold(f32::MAX, f32::min)
    };
    assert!(height_of("One line^two^three^four") > height_of("One line"));
}

#[test]
fn a_click_on_an_answer_of_a_dialog_is_found_by_the_panels_own_geometry() {
    let list = dialog(0, &["Yes", "No"]);
    let geometry = super::dialog::Geometry::of(&list.title, &list.message, 2);
    let on = |i: usize| glam::Vec2::new(0.0, geometry.rows_y[i]);
    assert_eq!(super::dialog::row_at(on(0), &list), Some(0));
    assert_eq!(super::dialog::row_at(on(1), &list), Some(1));
    assert_eq!(super::dialog::row_at(glam::Vec2::new(0.0, -230.0), &list), None, "outside the panel");
}

#[test]
fn a_key_row_shows_two_key_caps_and_lights_the_focused_one() {
    use crate::ui::theme::MenuValue;
    let row = |focus| MenuEntry {
        label: "Accelerate".into(),
        enabled: true,
        value: MenuValue::Keys { primary: "W".into(), secondary: "---".into(), focus },
    };
    let cell = MenuListCell::new();
    let bars = |focus| {
        let mut keys = list(MenuStyle::Sub, 0);
        keys.entries = vec![row(focus)];
        cell.set(ElementId::ControlsList.slot(), keys);
        let nodes = built(ElementId::ControlsList, &cell);
        assert!(texts(&nodes).contains(&"W") && texts(&nodes).contains(&"---"), "{:?}", texts(&nodes));
        let (w, dash) = (text_at(&nodes, "W"), text_at(&nodes, "---"));
        assert!(dash.world.w_axis.x > w.world.w_axis.x + 40.0, "the secondary cap is the right one");
        if focus != Some(1) {
            assert!(dash.colour[3] < w.colour[3], "an unbound key is dim");
        }
        nodes.len()
    };
    assert!(bars(Some(1)) > 0 && bars(None) > 0);
}

#[test]
fn the_credits_roll_scrolls_fades_at_its_ends_and_sets_headings_apart() {
    let cell = MenuListCell::new();
    let roll = |scroll: f32| MenuList {
        title: "Credits".into(),
        message: "\n\nEXECUTIVE PRODUCER\nJane Q. Public\nJohn Smith\n\nMUSIC\nSome Band".into(),
        style: MenuStyle::Credits,
        entries: Vec::new(),
        scroll,
        ..list(MenuStyle::Credits, 0)
    };
    cell.set(ElementId::Credits.slot(), roll(0.0));
    let nodes = built(ElementId::Credits, &cell);
    assert!(texts(&nodes).contains(&"CREDITS"));
    let heading = text_at(&nodes, "EXECUTIVE PRODUCER");
    let name = text_at(&nodes, "John Smith");
    assert_ne!(heading.colour[..3], name.colour[..3], "a heading is in the accent colour");
    assert!(name.world.w_axis.y > heading.world.w_axis.y, "the roll runs down the page");
    cell.set(ElementId::Credits.slot(), roll(0.5));
    let moved = built(ElementId::Credits, &cell);
    assert!(text_at(&moved, "John Smith").world.w_axis.y < name.world.w_axis.y, "a later scroll is further up");
}

#[test]
fn a_page_with_a_message_shows_it_at_the_right_of_the_title() {
    let cell = MenuListCell::new();
    let mut page = list(MenuStyle::Sub, 0);
    page.message = "Random order".into();
    cell.set(ElementId::Jukebox.slot(), page);
    let nodes = built(ElementId::Jukebox, &cell);
    let line = text_at(&nodes, "RANDOM ORDER");
    assert!(line.world.w_axis.x > text_at(&nodes, "MAIN MENU").world.w_axis.x + 100.0);
}
