//! Tests for the pursuit layout: the panel absent without a running chase, the cooldown bar only in Cooldown, the
//! search's last sighting, the detail levels and `pursuit_info`, and the end cards' timing.

use std::sync::Arc;

use blackbox_pursuit::EndReason;
use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

use super::PursuitPanel;
use super::end_card::{CARD_SECS, EndKind, card_opacity, end_kind};
use super::model::{PanelModel, Status};
use crate::hud::{HudState, LastKnown, PoliceIntel, PursuitEndCard, PursuitHud, PursuitPhase};
use crate::settings::hud::HudSettings;
use crate::ui::theme::custom::feed::{CustomFeed, CustomFeedCell};
use crate::ui::theme::custom::mode::Detail;
use crate::ui::theme::custom::tokens::{CustomLook, Motion, skin};

fn chase(phase: PursuitPhase) -> HudState {
    HudState {
        pursuit_phase: phase,
        vehicle_heat: 3.4,
        career_bounty: Some(12500),
        cops_engaged: 4,
        pursuit: Some(PursuitHud { secs: 134.0, evade: 0.4, cops_destroyed: 2, ..PursuitHud::default() }),
        police_intel: PoliceIntel {
            last_known: Some(LastKnown { position: [0.0, 0.0], age_secs: 8.6 }),
            cost_to_state: 25000,
            ..PoliceIntel::default()
        },
        ..HudState::default()
    }
}

#[test]
fn no_running_chase_no_panel() {
    for phase in [PursuitPhase::None, PursuitPhase::Escaped, PursuitPhase::Busted] {
        assert_eq!(PanelModel::of(&chase(phase), Detail::Advanced, true), None, "{phase:?}");
    }
}

#[test]
fn each_running_phase_names_its_status() {
    assert_eq!(Status::of(PursuitPhase::Active), Some(Status::Active));
    assert_eq!(Status::of(PursuitPhase::Search), Some(Status::Search));
    assert_eq!(Status::of(PursuitPhase::Cooldown), Some(Status::Cooldown));
}

#[test]
fn the_cooldown_bar_shows_only_in_cooldown() {
    let cooldown = PanelModel::of(&chase(PursuitPhase::Cooldown), Detail::Balanced, true).expect("a chase");
    assert_eq!(cooldown.cooldown, Some(0.4));
    for phase in [PursuitPhase::Active, PursuitPhase::Search] {
        let model = PanelModel::of(&chase(phase), Detail::Balanced, true).expect("a chase");
        assert_eq!(model.cooldown, None, "{phase:?}: the meter is not up");
    }
}

#[test]
fn search_says_when_the_police_last_saw_the_player() {
    let search = PanelModel::of(&chase(PursuitPhase::Search), Detail::Balanced, true).expect("a chase");
    assert_eq!(search.last_seen, Some(8.6));
    let active = PanelModel::of(&chase(PursuitPhase::Active), Detail::Balanced, true).expect("a chase");
    assert_eq!(active.last_seen, None, "tracking, not last known");
}

#[test]
fn the_detail_level_adds_fields() {
    let minimal = PanelModel::of(&chase(PursuitPhase::Active), Detail::Minimal, true).expect("a chase");
    assert_eq!(minimal.heat, Some(3));
    assert_eq!((minimal.duration, minimal.bounty, minimal.cost_to_state), (None, None, None));
    let balanced = PanelModel::of(&chase(PursuitPhase::Active), Detail::Balanced, true).expect("a chase");
    assert_eq!((balanced.duration, balanced.bounty), (Some(134.0), Some(12500)));
    assert_eq!(balanced.units, None);
    let advanced = PanelModel::of(&chase(PursuitPhase::Active), Detail::Advanced, true).expect("a chase");
    assert_eq!((advanced.cost_to_state, advanced.units), (Some(25000), Some((4, 2))));
}

#[test]
fn pursuit_info_off_keeps_the_status_and_the_cooldown_bar() {
    let model = PanelModel::of(&chase(PursuitPhase::Cooldown), Detail::Advanced, false).expect("a chase");
    assert_eq!(model.status, Status::Cooldown);
    assert_eq!(model.cooldown, Some(0.4));
    assert_eq!((model.heat, model.duration, model.bounty, model.cost_to_state), (None, None, None, None));
}

#[test]
fn a_high_heat_escalates() {
    let mut hot = chase(PursuitPhase::Active);
    hot.vehicle_heat = 4.2;
    assert!(PanelModel::of(&hot, Detail::Balanced, true).expect("a chase").high_heat());
    assert!(!PanelModel::of(&chase(PursuitPhase::Active), Detail::Balanced, true).expect("a chase").high_heat());
}

fn end(reason: EndReason, since_end: f32) -> PursuitEndCard {
    PursuitEndCard { reason, cops_destroyed: 2, secs: 75.0, since_end }
}

#[test]
fn the_escape_card_stays_three_seconds() {
    assert_eq!(end_kind(&end(EndReason::Evaded, 0.0), false), Some(EndKind::Escaped));
    assert_eq!(end_kind(&end(EndReason::Evaded, CARD_SECS - 0.1), false), Some(EndKind::Escaped));
    assert_eq!(end_kind(&end(EndReason::Evaded, CARD_SECS), false), None, "then back to free roam");
    let motion = Motion::FULL;
    assert_eq!(card_opacity(0.0, &motion), 0.0);
    assert_eq!(card_opacity(1.0, &motion), 1.0);
    assert!(card_opacity(CARD_SECS - 0.01, &motion) < 0.1, "faded out at its end");
}

#[test]
fn the_busted_card_waits_for_the_overlay() {
    assert_eq!(end_kind(&end(EndReason::Busted, 1.0), true), None, "the overlay is the screen");
    assert_eq!(end_kind(&end(EndReason::Busted, 1.0), false), Some(EndKind::Busted));
    assert_eq!(end_kind(&end(EndReason::None, 1.0), false), None);
}

/// The nodes the panel builds for `hud` on the second of two frames a second apart.
fn panel_nodes(hud: HudState) -> usize {
    let cell = CustomFeedCell::new();
    let hud = Arc::new(hud);
    let mut panel = PursuitPanel::new(cell.clone());
    let skin = skin(CustomLook::default(), [255, 200, 80, 255]);
    for now in [1.0, 2.0] {
        cell.set(CustomFeed {
            hud: Arc::clone(&hud),
            settings: HudSettings::default(),
            elapsed_s: now,
            ..CustomFeed::default()
        });
        panel.layout(&LayoutCx::new(&skin, now), Rect::SCREEN);
    }
    let mut out = DrawList::new();
    panel.build(&mut out);
    out.len()
}

#[test]
fn the_panel_draws_in_a_chase_and_nothing_without_one() {
    assert!(panel_nodes(chase(PursuitPhase::Active)) > 0);
    assert_eq!(panel_nodes(chase(PursuitPhase::None)), 0, "pursuit hidden when the phase is None");
    assert_eq!(panel_nodes(HudState::default()), 0);
}
