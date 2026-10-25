//! Tests for the Custom theme: which slots it fills, and what the radio card builds at each point of its life. They run
//! on synthetic state (a `RadioHud` fed by hand), with no game data and no GPU.

use blackbox_ui::{BuildCx, Component, DrawList, LayoutCx, NodeKind, Rect, Skin, SlotId, Theme, UiNode};

use super::feed::CustomFeed;
use super::layout::HudBox;
use super::radio_card::{RadioCard, SHOW_SECS, card_opacity};
use super::tokens::{self, Colours, CustomLook, Motion};
use super::{CustomFeedCell, CustomTheme, RadioFeedCell, palette};
use crate::audio::NowPlaying;
use crate::hud::RadioHud;
use crate::settings::HudLayout;
use crate::settings::hud::HudSettings;
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

/// The HUD options at full opacity, so a colour reads exactly as its token.
fn opaque() -> HudSettings {
    HudSettings { opacity: 1.0, ..HudSettings::default() }
}

fn skin() -> Skin {
    default_skin().merged(palette::bevy().skin()).merged(tokens::skin(CustomLook::default(), palette::bevy().secondary))
}

/// The nodes the radio card builds `age` seconds after `now` was announced, at the HUD options `settings` on a 1080p
/// window. Nothing when `now` is `None`, which is an idle radio.
fn card_at(now: Option<NowPlaying>, age: f32, settings: HudSettings) -> Vec<UiNode> {
    let mut hud = RadioHud::default();
    if let Some(now) = &now {
        hud.update(Some(now), 0.0);
        hud.update(Some(now), age);
    }
    let radio = RadioFeedCell::new();
    radio.set(&hud, SCREEN, HudLayout::Pc);
    let feed = CustomFeedCell::new();
    feed.set(CustomFeed { settings, screen: SCREEN, ..CustomFeed::default() });
    let mut card = RadioCard::new(radio, feed);
    let skin = skin();
    card.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    card.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<&str> {
    nodes.iter().filter_map(|node| node.text.as_deref()).collect()
}

fn text_at<'a>(nodes: &'a [UiNode], text: &str) -> &'a UiNode {
    nodes.iter().find(|n| n.text.as_deref() == Some(text)).unwrap_or_else(|| panic!("no {text:?} drawn"))
}

/// The left and top edges of the untextured nodes in `colour`'s hue, at any alpha (the plate's fill, as it fades).
fn plate_corner(nodes: &[UiNode], colour: [u8; 4]) -> (f32, f32) {
    let hue = |n: &&UiNode| n.colour[..3] == colour[..3];
    let fill = nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { texture: 0, .. })).filter(hue);
    fill.fold((f32::MAX, f32::MAX), |(left, top), n| {
        (left.min(n.world.w_axis.x - n.world.x_axis.x / 2.0), top.min(n.world.w_axis.y - n.world.y_axis.y / 2.0))
    })
}

#[test]
fn custom_fills_the_hud_the_menus_the_lan_screens_the_radio_card_and_the_map_and_falls_back_for_every_other_slot() {
    // The rewrite (ADR 0013) draws every HUD slot but the busted overlay, the radio card, the world map, the menus of
    // PRs 11a and 11b and the LAN screens with its own components; every other slot falls back to Original.
    let theme = CustomTheme::new(RadioFeedCell::default(), CustomFeedCell::default(), Default::default());
    let skin = default_skin();
    let cx = BuildCx::new(&skin);
    for element in ElementId::ALL {
        let filled = theme.element(element.slot(), &cx).is_some();
        let custom = matches!(
            element,
            ElementId::RadioCard
                | ElementId::WorldMapChrome
                | ElementId::LanMenu
                | ElementId::LanHud
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
        ) || super::menu::SLOTS.contains(element);
        assert_eq!(filled, custom, "{}", element.name());
    }
    assert!(theme.element(SlotId::new(9999), &cx).is_none(), "an id outside the catalogue has no Custom component");
}

#[test]
fn the_card_is_a_plate_and_three_rows_in_the_custom_tokens_while_up() {
    let nodes = card_at(Some(song(false)), 1.0, opaque());
    assert_eq!(texts(&nodes), ["Title", "Artist - Album", "1:05 / 3:20"]);
    let c = Colours::BRIEF;
    assert_eq!(text_at(&nodes, "Title").colour, c.text);
    assert_eq!(text_at(&nodes, "Artist - Album").colour, c.text_secondary);
    assert_eq!(text_at(&nodes, "1:05 / 3:20").colour, c.text_secondary);
    assert!(nodes.iter().any(|n| n.colour == c.surface), "the plate");
    assert!(nodes.iter().any(|n| n.colour == palette::bevy().secondary), "the plate's accent line");
    let y = |text: &str| text_at(&nodes, text).world.w_axis.y;
    assert!(y("Title") < y("Artist - Album") && y("Artist - Album") < y("1:05 / 3:20"), "one row under the other");
}

#[test]
fn a_paused_song_shows_the_state_on_the_time_line() {
    let nodes = card_at(Some(song(true)), 1.0, opaque());
    assert_eq!(texts(&nodes).last(), Some(&"1:05 / 3:20  PAUSED"));
}

#[test]
fn the_card_fades_in_and_out_on_the_motion_table() {
    let motion = Motion::FULL;
    let alpha = |age: f32| text_at(&card_at(Some(song(false)), age, opaque()), "Title").colour[3];
    assert!(alpha(motion.fade_in / 2.0) < 200, "fading in");
    assert_eq!(alpha(1.0), Colours::BRIEF.text[3], "up");
    let out = alpha(SHOW_SECS - motion.fade_out / 2.0);
    assert!(out > 0 && out < 200, "fading out: {out}");
    assert!((card_opacity(SHOW_SECS - motion.fade_out / 2.0, &motion) - 0.5).abs() < 1e-4);
    assert_eq!(card_opacity(motion.fade_in, &motion), 1.0);
}

#[test]
fn the_card_builds_nothing_when_idle_or_past_its_time() {
    assert!(card_at(None, 0.0, opaque()).is_empty(), "an idle radio");
    assert!(card_at(Some(song(false)), SHOW_SECS, opaque()).is_empty(), "the show time is over");
    assert!(!card_at(Some(song(false)), SHOW_SECS - 0.05, opaque()).is_empty(), "just before it is over, still drawn");
    assert!(card_at(Some(song(false)), 0.0, opaque()).is_empty(), "the first frame starts its fade from nothing");
}

#[test]
fn the_card_takes_the_hud_opacity() {
    let half = card_at(Some(song(false)), 1.0, HudSettings { opacity: 0.5, ..opaque() });
    let alpha = text_at(&half, "Title").colour[3];
    assert!((i32::from(alpha) - i32::from(Colours::BRIEF.text[3]) / 2).abs() <= 1, "{alpha}");
}

#[test]
fn the_card_sits_at_the_top_left_of_the_safe_area_and_slides_in_unless_motion_is_reduced() {
    let safe = HudBox::new(SCREEN, 1.0).safe();
    let surface = Colours::BRIEF.surface;
    let (left, top) = plate_corner(&card_at(Some(song(false)), 1.0, opaque()), surface);
    assert!((left - safe.min_x()).abs() < 0.05 && (top - safe.min_y()).abs() < 0.05, "({left}, {top})");
    let early = Motion::FULL.slide / 4.0;
    let (sliding, _) = plate_corner(&card_at(Some(song(false)), early, opaque()), surface);
    assert!(sliding < left, "slides in from the left");
    let still = HudSettings { motion_reduction: true, ..opaque() };
    let (reduced, _) = plate_corner(&card_at(Some(song(false)), early, still), surface);
    assert!((reduced - left).abs() < 0.05, "no slide with motion reduction");
}
