//! The LAN screens in both built-in themes, built with no game data and no GPU: what each draws of a page and of the session
//! HUD, and that the two put the rows in the same places (one hit test serves both).

use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, Rect, Theme, UiNode};

use super::custom::palette;
use super::lan_hud::fade_step;
use super::original::{FrameCell, OriginalTheme, lan as original_lan};
use super::remastered::{self, RemasteredTheme};
use super::{
    ElementId, LanHud, LanLine, LanLineKind, LanPage, LanSpectator, MenuEntry, MenuHint, MenuList, MenuStyle,
    MenuValue, default_skin,
};
use crate::input::Action;
use crate::ui::prompts::Prompt;

const ASPECT: f32 = 16.0 / 9.0;

fn page() -> LanPage {
    let rows = [
        ("Room name", MenuValue::Choice { text: "Ann's game_".into(), arrows: false }),
        ("Mode", MenuValue::Choice { text: "Circuit".into(), arrows: true }),
        ("Start Hosting", MenuValue::None),
    ];
    LanPage {
        list: MenuList {
            title: "Host a LAN game".into(),
            message: String::new(),
            entries: rows
                .into_iter()
                .map(|(label, value)| MenuEntry { label: label.into(), enabled: true, value, icon: None })
                .collect(),
            selected: 1,
            first: 0,
            hints: vec![MenuHint {
                keys: vec![Prompt::Key("Enter".into())],
                label: "Select / type".into(),
                action: Some(Action::MenuAccept),
            }],
            style: MenuStyle::Sub,
            nudge: None,
            scroll: 0.0,
            aspect: ASPECT,
        },
        subtitle: "Port 9750 as Ann".into(),
        error: Some("the port is taken".into()),
        side_title: "Chat".into(),
        side: vec!["Ann: hello".into(), "Bob joined".into()],
    }
}

fn hud() -> LanHud {
    LanHud {
        feed: vec![
            LanLine { kind: LanLineKind::Chat, text: "Ann: go go".into(), opacity: 1.0 },
            LanLine { kind: LanLineKind::Notice, text: "Bob left".into(), opacity: 0.3 },
        ],
        vote: Some("Vote: kick Bob  2 votes".into()),
        countdown: Some("Race starts in 3".into()),
        spectator: Some(LanSpectator {
            following: "Following Ann".into(),
            board: vec!["1 Ann".into(), "2 Bob".into()],
        }),
        aspect: ASPECT,
    }
}

/// The nodes `slot` draws under `theme` once the frame cell holds what `fill` puts in it.
fn built(theme: &dyn Theme, frame: &FrameCell, slot: ElementId, remastered: bool) -> Vec<UiNode> {
    let tokens = palette::remastered();
    let skin = match remastered {
        true => default_skin().merged(tokens.skin()).merged(remastered::palette_skin(&tokens)),
        false => default_skin(),
    };
    let _ = frame;
    let mut component: Box<dyn Component> =
        theme.element(slot.slot(), &BuildCx::new(&skin)).expect("the theme fills the slot");
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    out.iter().cloned().collect()
}

/// Both themes over one frame cell: `(original nodes, remastered nodes)` of `slot`.
fn both(slot: ElementId, fill: impl Fn(&FrameCell)) -> (Vec<UiNode>, Vec<UiNode>) {
    let frame = FrameCell::new();
    fill(&frame);
    let original = OriginalTheme::new(frame.clone());
    let remastered = RemasteredTheme::new(
        frame.clone(),
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    );
    (built(&original, &frame, slot, false), built(&remastered, &frame, slot, true))
}

fn lower(nodes: &[UiNode]) -> Vec<String> {
    nodes.iter().filter_map(|node| node.text.as_deref()).map(str::to_lowercase).collect()
}

fn text_y(nodes: &[UiNode], text: &str) -> f32 {
    let wanted = text.to_lowercase();
    nodes
        .iter()
        .find(|node| node.text.as_deref().is_some_and(|t| t.to_lowercase() == wanted))
        .unwrap_or_else(|| panic!("no {text:?} drawn"))
        .world
        .w_axis
        .y
}

#[test]
fn the_original_floor_fills_both_lan_slots() {
    let frame = FrameCell::new();
    let floor = OriginalTheme::new(frame);
    let skin = default_skin();
    for slot in [ElementId::LanMenu, ElementId::LanHud] {
        assert!(
            floor.element(slot.slot(), &BuildCx::new(&skin)).is_some(),
            "{} has an Original component",
            slot.name()
        );
    }
}

#[test]
fn both_themes_draw_a_page_with_its_rows_values_hints_and_side_column() {
    let (original, remastered) = both(ElementId::LanMenu, |frame| frame.lan().set_page(page()));
    for (theme, nodes) in [("Original", &original), ("Remastered", &remastered)] {
        let texts = lower(nodes);
        for wanted in [
            "host a lan game",
            "port 9750 as ann",
            "room name",
            "ann's game_",
            "mode",
            "circuit",
            "start hosting",
            "the port is taken",
            "chat",
            "ann: hello",
            "bob joined",
            "select / type",
            "enter",
        ] {
            assert!(texts.iter().any(|t| t == wanted), "{theme} draws {wanted:?}: {texts:?}");
        }
    }
}

#[test]
fn the_two_themes_put_each_row_where_the_other_does() {
    let (original, remastered) = both(ElementId::LanMenu, |frame| frame.lan().set_page(page()));
    for label in ["Room name", "Mode", "Start Hosting"] {
        let (a, b) = (text_y(&original, label), text_y(&remastered, label));
        assert!((a - b).abs() < 1e-3, "{label}: {a} against {b}");
    }
}

#[test]
fn nothing_is_drawn_while_the_cell_is_empty_or_holds_the_other_screen() {
    let (page_only, hud_only) = both(ElementId::LanHud, |frame| frame.lan().set_page(page()));
    assert!(page_only.is_empty() && hud_only.is_empty(), "a page leaves the HUD slot empty");
    let (menu_original, menu_remastered) = both(ElementId::LanMenu, |frame| frame.lan().set_hud(hud()));
    assert!(menu_original.is_empty() && menu_remastered.is_empty(), "the HUD leaves the page slot empty");
    let (empty_original, empty_remastered) = both(ElementId::LanMenu, |_| {});
    assert!(empty_original.is_empty() && empty_remastered.is_empty());
}

#[test]
fn both_themes_draw_the_session_hud() {
    let (original, remastered) = both(ElementId::LanHud, |frame| frame.lan().set_hud(hud()));
    for (theme, nodes) in [("Original", &original), ("Remastered", &remastered)] {
        let texts = lower(nodes);
        for wanted in [
            "ann: go go",
            "bob left",
            "race starts in 3",
            "vote: kick bob  2 votes",
            "spectating",
            "following ann",
            "1 ann",
            "2 bob",
        ] {
            assert!(texts.iter().any(|t| t == wanted), "{theme} draws {wanted:?}: {texts:?}");
        }
    }
}

#[test]
fn the_feed_draws_the_newest_line_lowest_and_fades_the_old_ones() {
    let (original, remastered) = both(ElementId::LanHud, |frame| frame.lan().set_hud(hud()));
    for nodes in [&original, &remastered] {
        assert!(text_y(nodes, "Bob left") > text_y(nodes, "Ann: go go"), "the second line is below the first");
    }
    assert!(fade_step(1.0) < fade_step(0.3), "a line at 0.3 is dimmer than a fresh one");
}

#[test]
fn a_hud_for_a_narrow_window_keeps_the_feed_at_its_left_edge() {
    let narrow = LanHud { aspect: 4.0 / 3.0, ..hud() };
    let wide = hud();
    let (_, narrow_nodes) = both(ElementId::LanHud, |frame| frame.lan().set_hud(narrow.clone()));
    let (_, wide_nodes) = both(ElementId::LanHud, |frame| frame.lan().set_hud(wide.clone()));
    let x = |nodes: &[UiNode]| nodes.iter().find(|n| n.text.as_deref() == Some("Ann: go go")).map(|n| n.world.w_axis.x);
    assert!(x(&narrow_nodes) > x(&wide_nodes), "the screen's left edge is nearer the middle in a narrower window");
}

#[test]
fn the_original_skin_names_the_games_fonts_and_every_fade_step() {
    let skin = original_lan::skin();
    assert!(skin.font(original_lan::FONT_TITLE_KEY).is_some() && skin.font(original_lan::FONT_BODY_KEY).is_some());
    for step in 0..super::lan_hud::FADE_STEPS {
        assert!(
            skin.colour(&original_lan::chat_key(step)).is_some()
                && skin.colour(&original_lan::notice_key(step)).is_some()
        );
    }
}
