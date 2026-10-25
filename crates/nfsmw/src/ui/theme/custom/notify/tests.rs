//! Tests for the Custom notification stack and feedback chip: where the plates go, how they fade and slide, and what
//! each draws, against the token skin with no game data and no GPU.

use std::sync::Arc;

use blackbox_ui::notify::Priority;
use blackbox_ui::{Component, DrawList, LayoutCx, NodeKind, Rect, Skin, UiNode};

use super::super::feed::{ChipView, CustomFeed, CustomFeedCell, NoticeView, NotificationView};
use super::super::layout::{Anchor, HudBox};
use super::super::text::Sizing;
use super::super::tokens::{Colours, CustomLook, Motion, SAFETY_OPACITY, fade, skin};
use super::Notifications;
use super::icon::{icon, known};
use super::lane::{self, CLUSTER_HEIGHT};
use super::stack::{self, MAX_PLATES, STACK_TOP};
use crate::hud::{ChipKind, HudState, NoticeCategory, NoticeId};
use crate::settings::hud::{HudPalette, HudSettings};
use crate::ui::present::Screen;

const GOLD: [u8; 4] = [255, 200, 80, 255];
const WIDE: Screen = Screen { width: 1920.0, height: 1080.0, pixels_per_point: 1.0 };

fn token_skin() -> Skin {
    skin(CustomLook { palette: HudPalette::Default, high_contrast: false }, GOLD)
}

/// A notice well into its life: entered, not yet leaving.
fn notice(priority: Priority, text: &str, secondary: Option<&str>) -> NoticeView {
    NoticeView {
        id: NoticeId::HeatUp,
        priority,
        category: NoticeCategory::Pursuit,
        text: text.to_owned(),
        secondary: secondary.map(str::to_owned),
        icon: "heat",
        age: 1.0,
        remaining: 2.0,
        count: 1,
    }
}

fn chip(kind: ChipKind, count: u32) -> ChipView {
    ChipView { kind, text: "Near miss".to_owned(), icon: "near-miss", age: 1.0, remaining: 1.0, count }
}

/// The nodes the `Notifications` component draws for `view` on a 16:9 window.
fn drawn(view: Option<NotificationView>, settings: HudSettings) -> Vec<UiNode> {
    let cell = CustomFeedCell::new();
    cell.set(CustomFeed {
        hud: Arc::new(HudState::default()),
        settings,
        screen: WIDE,
        notifications: view,
        ..CustomFeed::default()
    });
    let skin = token_skin();
    let mut component = Notifications::new(cell);
    component.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
    let mut out = DrawList::new();
    component.build(&mut out);
    out.iter().cloned().collect()
}

fn texts(nodes: &[UiNode]) -> Vec<String> {
    nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Text { .. }))
        .filter_map(|node| node.text.clone())
        .collect()
}

#[test]
fn no_view_and_an_empty_view_draw_nothing() {
    assert!(drawn(None, HudSettings::default()).is_empty());
    assert!(drawn(Some(NotificationView::default()), HudSettings::default()).is_empty());
}

#[test]
fn at_most_three_plates_stack_downwards_left_of_the_road() {
    let frame = HudBox::new(WIDE, 1.0);
    let sizing = Sizing::default();
    let notices: Vec<NoticeView> = (0..5).map(|n| notice(Priority::Medium, &format!("n{n}"), None)).collect();
    let rects = stack::rects(&notices, &frame, sizing);
    assert_eq!(rects.len(), MAX_PLATES);
    let content = frame.content();
    assert!((rects[0].min_y() - (content.min_y() + STACK_TOP * content.h)).abs() < 1e-3, "the stack starts at 36%");
    for pair in rects.windows(2) {
        assert!(pair[1].min_y() > pair[0].max_y(), "each plate is below the one before");
    }
    for rect in &rects {
        assert!(rect.min_x() >= frame.safe().min_x() - 1e-3, "inside the safe area");
        assert!(rect.max_x() < content.min_x() + content.w / 3.0, "never over the road's central third");
    }
    let drawn = drawn(Some(NotificationView { notices, ..NotificationView::default() }), HudSettings::default());
    assert_eq!(texts(&drawn), ["n0", "n1", "n2"], "three plates, top down");
}

#[test]
fn a_second_line_makes_the_plate_taller() {
    let frame = HudBox::default();
    let one = stack::rects(&[notice(Priority::Medium, "a", None)], &frame, Sizing::default());
    let two = stack::rects(&[notice(Priority::Medium, "a", Some("b"))], &frame, Sizing::default());
    assert!(two[0].h > one[0].h);
}

#[test]
fn a_critical_notice_is_a_solid_critical_plate_with_dark_text_and_its_icon() {
    let settings = HudSettings { opacity: 0.3, ..HudSettings::default() };
    let view = NotificationView {
        notices: vec![notice(Priority::Critical, "Roadblock reported", None)],
        ..NotificationView::default()
    };
    let nodes = drawn(Some(view), settings);
    let floor = fade(Colours::BRIEF.critical, SAFETY_OPACITY);
    assert!(nodes.iter().any(|node| node.colour == floor), "the plate keeps the safety opacity at opacity 0.3");
    let dark = fade(Colours::BRIEF.critical_text, SAFETY_OPACITY);
    let text = nodes.iter().find(|node| matches!(node.kind, NodeKind::Text { .. })).expect("the text");
    assert_eq!(text.colour, dark, "dark text on the critical plate");
    let icon_quads = nodes.iter().filter(|node| node.colour == dark && !matches!(node.kind, NodeKind::Text { .. }));
    assert!(icon_quads.count() > 0, "the icon is drawn in the dark text colour");
}

#[test]
fn every_catalogue_icon_has_a_pictogram_that_draws() {
    let ids = [
        "heat",
        "roadblock",
        "helicopter",
        "eye-off",
        "timer",
        "escaped",
        "busted",
        "checkpoint",
        "wrong-way",
        "lap",
        "position",
        "near-miss",
        "speed-trap",
        "bounty",
        "tyre",
        "damage",
        "warning",
        "nitrous",
    ];
    for id in ids {
        assert!(known(id), "{id} has no pictogram");
        let mut out = DrawList::new();
        icon(id, Rect::new(0.0, 0.0, 14.0, 14.0), GOLD, 0.5).build(&mut out);
        assert!(!out.is_empty(), "{id} draws nothing");
        assert!(out.iter().all(|node| node.colour == GOLD));
    }
    let mut out = DrawList::new();
    icon("no-such-icon", Rect::new(0.0, 0.0, 14.0, 14.0), GOLD, 0.5).build(&mut out);
    assert!(!out.is_empty(), "an unknown id still draws its fallback, never nothing");
}

#[test]
fn a_notice_fades_in_and_out_and_a_repeat_does_not_replay_its_entry() {
    let motion = Motion::FULL;
    let mut entering = notice(Priority::Medium, "a", None);
    entering.age = 0.0;
    assert_eq!(stack::opacity(&entering, motion, 0.9), 0.0, "it starts invisible");
    // A repeat folded in keeps the age and pushes the end out: the plate stays at full opacity.
    let refreshed = NoticeView { age: 2.5, remaining: 4.0, count: 2, ..entering.clone() };
    assert!((stack::opacity(&refreshed, motion, 0.9) - 0.9).abs() < 1e-6);
    let leaving = NoticeView { remaining: motion.fade_out / 2.0, ..refreshed };
    assert!((stack::opacity(&leaving, motion, 0.9) - 0.45).abs() < 1e-3, "half way out");
}

#[test]
fn notices_slide_in_from_the_left_and_only_fade_with_motion_reduced() {
    assert!(stack::slide(0.0, Motion::FULL, 1.0) < 0.0, "it starts left of its place");
    assert_eq!(stack::slide(Motion::FULL.slide, Motion::FULL, 1.0), 0.0, "and ends in it");
    assert_eq!(stack::slide(0.0, Motion::REDUCED, 1.0), 0.0, "motion reduced: no travel");
    assert!(lane::rise(0.0, Motion::FULL, 1.0) > 0.0);
    assert_eq!(lane::rise(0.0, Motion::REDUCED, 1.0), 0.0);
}

#[test]
fn the_count_badge_shows_the_repeats() {
    let mut repeated = notice(Priority::Medium, "Near miss", None);
    repeated.count = 3;
    let view = NotificationView { notices: vec![repeated], ..NotificationView::default() };
    let shown = texts(&drawn(Some(view), HudSettings::default()));
    assert_eq!(shown, ["Near miss", "×3"]);
    let once = NotificationView { notices: vec![notice(Priority::Medium, "Near miss", None)], ..Default::default() };
    assert_eq!(texts(&drawn(Some(once), HudSettings::default())), ["Near miss"], "no badge for one");
}

#[test]
fn the_chip_sits_above_the_cluster_with_its_count() {
    let frame = HudBox::new(WIDE, 1.0);
    let rect = lane::rect(&frame);
    let (_, bottom) = frame.point(Anchor::BottomRight);
    assert!(rect.max_y() <= bottom - CLUSTER_HEIGHT + 1e-3, "clear of the cluster");
    assert!((rect.max_x() - frame.safe().max_x()).abs() < 1e-3, "on the cluster's right edge");
    let view = NotificationView { chip: Some(chip(ChipKind::NearMiss, 3)), ..NotificationView::default() };
    assert_eq!(texts(&drawn(Some(view), HudSettings::default())), ["Near miss", "×3"]);
}
