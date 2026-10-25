//! Tests for the Custom LAN screens: the page (`LanMenu`) and the session HUD (`LanHud`), built from a fixed LAN cell
//! with no game data and no GPU.

use blackbox_ui::{BuildCx, DrawList, LayoutCx, Rect, Skin, Theme, UiNode};

use super::feed::{CustomFeed, CustomFeedCell};
use super::menu::MenuLook;
use super::menu::layout::MenuFrame;
use super::tokens::{self, Colours, CustomLook};
use super::{CustomTheme, RadioFeedCell, palette};
use crate::settings::hud::HudSettings;
use crate::ui::prompts::Prompt;
use crate::ui::theme::{
    DEFAULT_ASPECT, ElementId, LanCell, LanHud, LanLine, LanLineKind, LanPage, LanSpectator, MenuEntry, MenuHint,
    MenuList, MenuStyle, MenuValue, default_skin,
};

fn page() -> LanPage {
    let choice = |label: &str, value: &str| MenuEntry {
        value: MenuValue::Choice { text: value.into(), arrows: true },
        ..MenuEntry::plain(label, true)
    };
    LanPage {
        list: MenuList {
            title: "Host a LAN game".into(),
            message: String::new(),
            entries: vec![
                choice("Room name", "Ann's game_"),
                choice("Mode", "Circuit"),
                MenuEntry::plain("Start", true),
            ],
            selected: 1,
            first: 0,
            hints: vec![MenuHint { keys: vec![Prompt::Key("Enter".into())], label: "Select".into(), action: None }],
            style: MenuStyle::Sub,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        },
        subtitle: "Port 9750 as Ann".into(),
        error: Some("The port is taken".into()),
        side_title: "Chat".into(),
        side: vec!["Ann: hello".into(), "Bob joined".into()],
    }
}

fn hud() -> LanHud {
    LanHud {
        feed: vec![
            LanLine { kind: LanLineKind::Chat, text: "Ann: go go".into(), opacity: 1.0 },
            LanLine { kind: LanLineKind::Notice, text: "Bob left".into(), opacity: 0.4 },
        ],
        vote: Some("Vote: restart".into()),
        countdown: Some("Race starts in 3".into()),
        spectator: Some(LanSpectator { following: "Following Ann".into(), board: vec!["1 Ann".into()] }),
        aspect: DEFAULT_ASPECT,
    }
}

fn skin() -> Skin {
    default_skin()
        .merged(palette::bevy().skin())
        .merged(tokens::skin(CustomLook::default(), palette::bevy().secondary))
        .merged(MenuLook::default().skin())
}

/// What the Custom theme draws on `slot` from `lan`, with the HUD options `settings`.
fn built(slot: ElementId, lan: &LanCell, settings: HudSettings) -> Vec<UiNode> {
    let feed = CustomFeedCell::new();
    feed.set(CustomFeed { settings, ..CustomFeed::default() });
    let theme = CustomTheme::new(RadioFeedCell::default(), feed, Default::default()).with_lan(lan.clone());
    let skin = skin();
    let mut component = theme.element(slot.slot(), &BuildCx::new(&skin)).expect("Custom draws the LAN slots");
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    out.iter().cloned().collect()
}

/// The HUD options at full opacity, so a colour reads exactly as its token.
fn opaque() -> HudSettings {
    HudSettings { opacity: 1.0, ..HudSettings::default() }
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

fn text_at<'a>(nodes: &'a [UiNode], text: &str) -> &'a UiNode {
    nodes.iter().find(|n| n.text.as_deref() == Some(text)).unwrap_or_else(|| panic!("no {text:?} drawn"))
}

#[test]
fn the_page_is_a_custom_list_with_its_status_failure_and_chat() {
    let lan = LanCell::default();
    lan.set_page(page());
    let nodes = built(ElementId::LanMenu, &lan, HudSettings::default());
    let shown = texts(&nodes);
    for text in [
        "HOST A LAN GAME",
        "Port 9750 as Ann",
        "Room name",
        "Ann's game_",
        "Mode",
        "Circuit",
        "Start",
        "The port is taken",
        "CHAT",
        "Ann: hello",
        "Bob joined",
        "Enter",
        "Select",
    ] {
        assert!(shown.contains(&text), "{text:?} is drawn: {shown:?}");
    }
    let frame = MenuFrame::new(DEFAULT_ASPECT, 1.0);
    for (i, label) in ["Room name", "Mode", "Start"].into_iter().enumerate() {
        assert!((text_at(&nodes, label).world.w_axis.y - frame.row_y(i)).abs() < 1e-3, "{label} on the Custom row");
    }
    assert_eq!(text_at(&nodes, "The port is taken").colour, Colours::BRIEF.danger, "the failure in the danger colour");
    assert!(
        text_at(&nodes, "Ann: hello").world.w_axis.x > text_at(&nodes, "Mode").world.w_axis.x,
        "the chat is right of the rows"
    );
}

#[test]
fn the_side_column_gives_way_in_a_narrow_window() {
    let lan = LanCell::default();
    let narrow = LanPage { list: MenuList { aspect: 1.0, ..page().list }, ..page() };
    lan.set_page(narrow);
    let nodes = built(ElementId::LanMenu, &lan, HudSettings::default());
    assert!(!texts(&nodes).contains(&"Ann: hello"), "no room for the chat");
    assert!(texts(&nodes).contains(&"Room name"), "the rows stay");
}

#[test]
fn nothing_is_drawn_while_the_cell_is_empty_or_holds_the_other_screen() {
    let lan = LanCell::default();
    assert!(built(ElementId::LanMenu, &lan, HudSettings::default()).is_empty());
    assert!(built(ElementId::LanHud, &lan, HudSettings::default()).is_empty());
    lan.set_page(page());
    assert!(built(ElementId::LanHud, &lan, HudSettings::default()).is_empty(), "a page leaves the HUD slot empty");
    lan.set_hud(hud());
    assert!(built(ElementId::LanMenu, &lan, HudSettings::default()).is_empty(), "the HUD leaves the page slot empty");
}

#[test]
fn the_session_hud_draws_the_feed_the_banners_and_the_spectator_panel() {
    let lan = LanCell::default();
    lan.set_hud(hud());
    let nodes = built(ElementId::LanHud, &lan, opaque());
    let shown = texts(&nodes);
    for text in ["Ann: go go", "Bob left", "Vote: restart", "Race starts in 3", "SPECTATING", "Following Ann", "1 Ann"]
    {
        assert!(shown.contains(&text), "{text:?} is drawn: {shown:?}");
    }
    let at = |text: &str| text_at(&nodes, text).world.w_axis;
    assert!(at("Ann: go go").y > 0.0 && at("Ann: go go").x < 0.0, "the feed is at the bottom left");
    assert!(at("Bob left").y > at("Ann: go go").y, "the newest line lowest");
    assert!(at("Following Ann").y < 0.0 && at("Following Ann").x > 0.0, "the spectator panel is at the top right");
    assert!(at("Race starts in 3").y < at("Vote: restart").y, "the countdown above the vote");
    let chat = text_at(&nodes, "Ann: go go").colour;
    let notice = text_at(&nodes, "Bob left").colour;
    assert_eq!(chat, Colours::BRIEF.text);
    assert_eq!(notice[..3], Colours::BRIEF.text_secondary[..3], "a notice is in the secondary colour");
    assert!(notice[3] < Colours::BRIEF.text_secondary[3], "and keeps its fade");
}

#[test]
fn the_session_hud_takes_the_hud_opacity() {
    let lan = LanCell::default();
    lan.set_hud(hud());
    let full = built(ElementId::LanHud, &lan, opaque());
    let half = built(ElementId::LanHud, &lan, HudSettings { opacity: 0.5, ..opaque() });
    let alpha = |nodes: &[UiNode]| text_at(nodes, "Ann: go go").colour[3];
    assert!(alpha(&half) < alpha(&full), "{} against {}", alpha(&half), alpha(&full));
}
