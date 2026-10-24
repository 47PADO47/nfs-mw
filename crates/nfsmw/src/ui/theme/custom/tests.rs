//! Tests for the Custom theme: which slots it fills, and what the radio card builds at each point of its life. They run
//! on synthetic state (a `RadioHud` fed by hand), with no game data and no GPU.

use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, NodeKind, Rect, SlotId, Theme, UiNode};

use super::palette;
use super::radio_card::{FADE_SECS, RadioCard, SHOW_SECS};
use super::{CustomFeedCell, CustomTheme, RadioFeedCell, card_skin};
use crate::audio::NowPlaying;
use crate::hud::RadioHud;
use crate::settings::HudLayout;
use crate::ui::present::Screen;
use crate::ui::theme::{ElementId, default_skin};

const SCREEN: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

fn song(paused: bool) -> NowPlaying {
    NowPlaying {
        song: 3,
        artist: "Artist".into(),
        title: "Title".into(),
        album: "Album".into(),
        elapsed_secs: 65.0,
        length_secs: 200.0,
        paused,
        serial: 1,
    }
}

/// The nodes the radio card builds `age` seconds after `now` was announced, on the 16:9 PC layout. Nothing when `now`
/// is `None`, which is an idle radio.
fn card_at(now: Option<NowPlaying>, age: f32) -> Vec<UiNode> {
    let mut hud = RadioHud::default();
    if let Some(now) = &now {
        hud.update(Some(now), 0.0);
        hud.update(Some(now), age);
    }
    let feed = RadioFeedCell::new();
    feed.set(&hud, SCREEN, HudLayout::Pc);
    let mut card = RadioCard::new(feed);
    let skin = card_skin().merged(palette::bevy().skin());
    card.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    card.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

#[test]
fn custom_fills_the_hud_the_menus_the_radio_card_and_the_map_legend_and_falls_back_for_every_other_slot() {
    // The rewrite (ADR 0013) draws every HUD slot but the busted overlay, the radio card, the map legend and the menus
    // of PR 11a with its own components; every other slot falls back to Original.
    let theme = CustomTheme::new(RadioFeedCell::default(), CustomFeedCell::default(), Default::default());
    let skin = default_skin();
    let cx = BuildCx::new(&skin);
    for element in ElementId::ALL {
        let filled = theme.element(element.slot(), &cx).is_some();
        let custom = matches!(
            element,
            ElementId::RadioCard
                | ElementId::WorldMapChrome
                | ElementId::Speedometer
                | ElementId::Tachometer
                | ElementId::GearIndicator
                | ElementId::Nitrous
                | ElementId::Turbo
                | ElementId::Speedbreaker
                | ElementId::WorldMarkers
                | ElementId::Notifications
                | ElementId::RaceStatus
                | ElementId::Leaderboard
                | ElementId::Banner
                | ElementId::PursuitMeter
                | ElementId::BountyRow
                | ElementId::PoliceRadar
                | ElementId::VehicleCondition
                | ElementId::Minimap
                | ElementId::Blips
                | ElementId::MainMenu
                | ElementId::Categories
                | ElementId::PauseMenu
                | ElementId::SafeHouseMenu
                | ElementId::OptionTable
                | ElementId::Dialog
        );
        assert_eq!(filled, custom, "{}", element.name());
    }
    assert!(theme.element(SlotId::new(9999), &cx).is_none(), "an id outside the catalogue has no Custom component");
}

#[test]
fn the_card_is_a_panel_and_three_text_rows_at_full_alpha_while_up() {
    let nodes = card_at(Some(song(false)), 0.5);
    assert_eq!(nodes.len(), 4, "one background, then title, credit and time");
    assert!(matches!(nodes[0].kind, NodeKind::Image { texture: 0, .. }));
    assert_eq!(texts(&nodes), ["Title", "Artist - Album", "1:05 / 3:20"]);
    assert_eq!(nodes[0].colour, [0, 0, 0, 170]);
    assert_eq!(nodes[1].colour, [255, 255, 255, 255]);
    // The credit and time lines both take the palette's muted token.
    assert_eq!(nodes[2].colour, palette::bevy().muted);
    assert_eq!(nodes[3].colour, palette::bevy().muted);
}

#[test]
fn a_paused_song_shows_the_state_on_the_time_line() {
    let nodes = card_at(Some(song(true)), 0.0);
    assert_eq!(texts(&nodes).last(), Some(&"1:05 / 3:20  PAUSED"));
}

#[test]
fn the_card_fades_over_its_last_second() {
    let nodes = card_at(Some(song(false)), SHOW_SECS - FADE_SECS / 2.0);
    assert_eq!(nodes.len(), 4, "still up, at half alpha");
    assert!((i32::from(nodes[0].colour[3]) - 85).abs() <= 1, "background {:?}", nodes[0].colour);
    assert!((i32::from(nodes[1].colour[3]) - 128).abs() <= 1, "title {:?}", nodes[1].colour);
}

#[test]
fn the_card_builds_nothing_when_idle_or_past_its_fade() {
    assert!(card_at(None, 0.0).is_empty(), "an idle radio");
    assert!(card_at(Some(song(false)), SHOW_SECS).is_empty(), "the show time is over");
    assert!(!card_at(Some(song(false)), SHOW_SECS - 0.05).is_empty(), "just before it is over, still drawn");
}

#[test]
fn the_card_sits_where_the_hud_layout_puts_it() {
    // 1080p, 16:9, PC layout: the card's top-left corner is (-420, -228) in FEng units, and its box is 242 by 60
    // units. The background is inset by the palette's padding, so its centre is at (-305, -204) plus that padding.
    let nodes = card_at(Some(song(false)), 0.0);
    let centre = nodes[0].world.w_axis;
    let padding = palette::bevy().padding;
    let (x, y) = (-305.0 + padding, -204.0 + padding);
    assert!((centre.x - x).abs() < 1e-3 && (centre.y - y).abs() < 1e-3, "{centre:?}");
}
