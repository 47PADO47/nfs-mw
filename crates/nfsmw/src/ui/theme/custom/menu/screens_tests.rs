//! Tests for the Custom list screens of PR 11b: the key list, the jukebox, the credits, the Blacklist and a rival's bio.
//! Each builds from a fixed list, with no game data and no GPU, through the Custom theme's component for its slot.

use blackbox_ui::{NodeKind, UiNode};
use glam::Vec2;

use super::blacklist::{BlacklistFrame, row_at as blacklist_row_at};
use super::keys::cap_rects;
use super::layout::{BAR_WIDTH, MenuFrame};
use super::tests::{ASPECT, accent, built, list, panels_at, text_at, texts};
use crate::ui::theme::custom::tokens::Colours;
use crate::ui::theme::{ElementId, MenuEntry, MenuList, MenuStyle, MenuValue, RivalInfo, RivalStatus};

fn keys_row(label: &str, primary: &str, secondary: &str, focus: usize) -> MenuEntry {
    let value = MenuValue::Keys { primary: primary.into(), secondary: secondary.into(), focus: Some(focus) };
    MenuEntry { value, ..MenuEntry::plain(label, true) }
}

fn controls(selected: usize) -> MenuList {
    MenuList {
        title: "Controls".into(),
        entries: vec![
            keys_row("Gas", "W", "Up", 1),
            keys_row("Brake", "S", "---", 1),
            keys_row("Nitrous", "N", "X", 1),
        ],
        ..list(MenuStyle::Sub, selected)
    }
}

/// The untextured panels in `colour` whose middle is within `half` of `(x, y)` on both axes.
fn panels_near(nodes: &[UiNode], colour: [u8; 4], (x, y): (f32, f32), half: f32) -> usize {
    nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Image { texture: 0, .. }) && n.colour == colour)
        .filter(|n| (n.world.w_axis.x - x).abs() < half && (n.world.w_axis.y - y).abs() < half)
        .count()
}

#[test]
fn the_key_list_shows_each_control_with_two_caps_and_marks_the_focused_one() {
    let nodes = built(ElementId::ControlsList, controls(0));
    for text in ["CONTROLS", "Gas", "Brake", "Nitrous", "W", "Up", "S", "---", "N", "X"] {
        assert!(texts(&nodes).contains(&text), "{text:?} is drawn");
    }
    let frame = MenuFrame::new(ASPECT, 1.0);
    let x = |text: &str| text_at(&nodes, text).world.w_axis.x;
    assert!(x("W") < x("Up"), "the primary binding on the left, the secondary on the right");
    assert_eq!(x("W"), x("S"), "one column per binding");
    let [primary, secondary] = cap_rects(frame.value_column(frame.row_y(0)), frame.scale());
    let half = secondary.w / 2.0;
    assert_eq!(panels_near(&nodes, accent(), (secondary.x, secondary.max_y()), half), 1, "the focused cap's line");
    assert_eq!(panels_near(&nodes, accent(), (primary.x, primary.max_y()), half), 0);
    let [_, on_brake] = cap_rects(frame.value_column(frame.row_y(1)), frame.scale());
    // Within half a row of the brake cap's line, so the focused line one row up is not counted.
    let near = half.min((frame.row_y(1) - frame.row_y(0)).abs() / 2.0);
    assert_eq!(panels_near(&nodes, accent(), (on_brake.x, on_brake.max_y()), near), 0, "only the selected row has one");
    assert!(text_at(&nodes, "---").colour[3] < text_at(&nodes, "S").colour[3], "an unbound key is faint");
}

#[test]
fn the_key_list_in_the_pause_menu_dims_the_whole_window() {
    let window = MenuFrame::new(ASPECT, 1.0).window();
    let nodes = built(ElementId::ControlsList, MenuList { style: MenuStyle::Pause, ..controls(0) });
    let widest = nodes.iter().map(|n| n.world.x_axis.x).fold(0.0_f32, f32::max);
    assert!((widest - window.w).abs() < 1e-3);
}

#[test]
fn the_playlist_shows_its_songs_and_modes_and_the_order_beside_the_title() {
    let song = |label: &str, mode: &str| MenuEntry {
        value: MenuValue::Choice { text: mode.into(), arrows: false },
        ..MenuEntry::plain(label, true)
    };
    let playlist = MenuList {
        title: "EA Trax".into(),
        message: "Sequential order".into(),
        entries: vec![song("Artist - First", "Play"), song("Band - Second", "Skip")],
        ..list(MenuStyle::Sub, 1)
    };
    let nodes = built(ElementId::Jukebox, playlist);
    for text in ["EA TRAX", "Artist - First", "Band - Second", "Play", "Skip", "Sequential order"] {
        assert!(texts(&nodes).contains(&text), "{text:?} is drawn");
    }
    let frame = MenuFrame::new(ASPECT, 1.0);
    let order = text_at(&nodes, "Sequential order").world.w_axis.y;
    assert!(order < frame.list_top(), "the order line is up by the title, not on the description line");
    assert_eq!(panels_at(&nodes, accent(), frame.row_y(1)), 1, "the selected song's bar");
}

#[test]
fn the_credits_roll_fades_at_its_ends_and_marks_its_headings() {
    let mut lines: Vec<String> =
        ["Leaving", "Fading", "Still", "EXECUTIVE PRODUCER", "Jane Doe"].map(String::from).into();
    lines.extend((5..16).map(|i| format!("Name {i}")));
    let roll = MenuList { title: "Credits".into(), message: lines.join("\n"), ..list(MenuStyle::Credits, 0) };
    let nodes = built(ElementId::Credits, roll);
    let shown = texts(&nodes);
    assert!(shown.contains(&"CREDITS"), "the title");
    assert!(!shown.contains(&"Leaving"), "the line at the top has faded out");
    assert!(!shown.contains(&"Name 15"), "a line below the roll has not come in yet");
    assert!(text_at(&nodes, "Fading").colour[3] < 255, "a line near the top is fading");
    assert_eq!(text_at(&nodes, "EXECUTIVE PRODUCER").colour, accent(), "a heading is in the accent colour");
    assert_eq!(text_at(&nodes, "Jane Doe").colour, Colours::BRIEF.text);
    let x = |text: &str| text_at(&nodes, text).world.w_axis.x;
    assert_eq!(x("Jane Doe"), x("EXECUTIVE PRODUCER"), "the roll is one centred column");
}

fn rival(rank: u8, status: RivalStatus) -> MenuEntry {
    let info = RivalInfo {
        rank,
        portrait: 1000 + u32::from(rank),
        alias: "Baron".into(),
        car: "Porsche".into(),
        strength: "Infractions".into(),
        bounty: format!("{rank},000"),
        status,
    };
    MenuEntry { label: format!("Rival {rank}"), enabled: true, value: MenuValue::Rival(info), icon: None }
}

fn blacklist(selected: usize, aspect: f32) -> MenuList {
    let status = |rank: u8| match rank {
        11..=15 => RivalStatus::Complete,
        10 => RivalStatus::InProgress,
        _ => RivalStatus::Locked,
    };
    MenuList {
        title: "Blacklist".into(),
        message: "5/15".into(),
        entries: (1..=15u8).rev().map(|rank| rival(rank, status(rank))).collect(),
        aspect,
        ..list(MenuStyle::Blacklist, selected)
    }
}

fn pictures(nodes: &[UiNode], texture: u32) -> usize {
    nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture: t, .. } if t == texture)).count()
}

#[test]
fn every_rival_has_a_row_with_its_rank_name_bounty_status_and_picture() {
    let nodes = built(ElementId::Blacklist, blacklist(5, ASPECT));
    let drawn = texts(&nodes);
    for rank in 1..=15u8 {
        assert!(drawn.contains(&format!("Rival {rank}").as_str()), "rival {rank}");
        assert!(drawn.contains(&format!("{rank},000").as_str()), "bounty {rank}");
    }
    assert!(drawn.contains(&"BLACKLIST") && drawn.contains(&"5/15"));
    assert_eq!(drawn.iter().filter(|text| **text == "Complete").count(), 5);
    assert_eq!(drawn.iter().filter(|text| **text == "Locked").count(), 9);
    assert_eq!(drawn.iter().filter(|text| **text == "In progress").count(), 2, "its row and the card");
    assert_eq!(pictures(&nodes, 1001), 1, "an unselected rival's picture is in its row only");
    assert_eq!(pictures(&nodes, 1010), 2, "the selected rival's picture is in its row and on the card");
}

#[test]
fn the_selected_rival_is_on_the_raised_plate_and_its_card_shows_the_facts() {
    let rows = BlacklistFrame::of(&MenuFrame::new(ASPECT, 1.0));
    let nodes = built(ElementId::Blacklist, blacklist(0, ASPECT));
    // The bar is at the rows' left edge; the card's picture frame (also in the accent colour) is far to the right.
    let bar = |i: usize| panels_near(&nodes, accent(), (rows.left() + BAR_WIDTH / 2.0, rows.row_y(i)), 2.0);
    assert_eq!(bar(0), 1, "the bar on the selected row");
    assert_eq!(bar(1), 0, "and on no other");
    assert!(panels_at(&nodes, Colours::BRIEF.surface_raised, rows.row_y(0)) > 0, "the raised plate");
    let drawn = texts(&nodes);
    for text in ["BOUNTY", "STATUS", "Baron"] {
        assert!(drawn.contains(&text), "the card shows {text:?}");
    }
    assert_eq!(drawn.iter().filter(|text| **text == "Rival 15").count(), 2, "row and card");
    assert!(text_at(&nodes, "Rival 5").colour[3] < text_at(&nodes, "Rival 15").colour[3], "a locked rival is faint");
}

#[test]
fn the_card_gives_way_in_a_square_window() {
    assert!(BlacklistFrame::of(&MenuFrame::new(ASPECT, 1.0)).card().is_some());
    assert!(BlacklistFrame::of(&MenuFrame::new(1.0, 1.0)).card().is_none());
    let nodes = built(ElementId::Blacklist, blacklist(0, 1.0));
    assert!(!texts(&nodes).contains(&"BOUNTY"), "no card");
    assert_eq!(texts(&nodes).iter().filter(|text| **text == "Rival 15").count(), 1, "the row only");
}

#[test]
fn the_fifteen_rows_clear_the_footer_and_a_point_finds_its_row() {
    for text_size in [1.0, 1.5] {
        let frame = MenuFrame::new(ASPECT, text_size);
        let rows = BlacklistFrame::of(&frame);
        assert!(rows.row_y(14) + rows.row_rect(rows.row_y(14)).h / 2.0 <= rows.bottom() + 1e-3, "{text_size}");
        assert!(rows.bottom() < frame.footer_y(), "the rows end above the footer line");
    }
    let list = blacklist(0, ASPECT);
    let rows = BlacklistFrame::of(&MenuFrame::new(ASPECT, 1.0));
    let at = |x: f32, i: usize| blacklist_row_at(Vec2::new(x, rows.row_y(i)), &list, ASPECT, 1.0);
    assert_eq!(at(rows.left() + 50.0, 0), Some(0));
    assert_eq!(at(rows.left() + 50.0, 14), Some(14));
    assert_eq!(at(rows.right() + 2.0, 3), None, "right of the rows");
    let below = Vec2::new(rows.left() + 50.0, rows.row_y(14) + 40.0);
    assert_eq!(blacklist_row_at(below, &list, ASPECT, 1.0), None, "below the last rival");
}

#[test]
fn the_bio_page_shows_the_rival_its_facts_and_the_wrapped_bio() {
    let bio = "He is a rich guy from the coast and he thinks garage custom is an excuse for being broke. ".repeat(3);
    let page = MenuList {
        title: "Blacklist 10".into(),
        message: bio,
        entries: vec![rival(10, RivalStatus::InProgress)],
        style: MenuStyle::Rival,
        ..blacklist(0, ASPECT)
    };
    let nodes = built(ElementId::Blacklist, page);
    let drawn = texts(&nodes);
    for text in ["BLACKLIST 10", "Rival 10", "ALIAS", "CAR", "STRENGTH", "Baron", "Porsche", "Infractions"] {
        assert!(drawn.contains(&text), "{text:?}");
    }
    let wrapped = drawn.iter().filter(|line| line.contains("garage") || line.starts_with("He is")).count();
    assert!(wrapped >= 2, "the bio is wrapped into lines: {drawn:?}");
    assert_eq!(pictures(&nodes, 1010), 1);
}
