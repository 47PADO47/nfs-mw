//! Tests for the police threat indicators (`threats.rs`): fair knowledge, the cap, the last-known fade and style, and
//! the edge arrows.

use glam::Vec3;

use super::project::edge_rect;
use super::tests::car_at_origin;
use super::threats::{
    LAST_KNOWN_ALPHA, LAST_KNOWN_FADE, MAX_THREATS, ThreatKind, ThreatStyle, last_known_alpha, place, threats,
};
use crate::hud::{HelicopterIntel, HudCamera, HudState, LastKnown, PursuitPhase};
use crate::settings::hud::HudSettings;
use crate::ui::present::Screen;
use crate::ui::theme::custom::layout::window;
use crate::viewer::camera::Lens;

/// The car at the origin heading along +x, in a chase.
fn chase() -> HudState {
    let mut hud = car_at_origin();
    hud.pursuit_phase = PursuitPhase::Active;
    hud.world_marks.car_z = Some(0.0);
    hud
}

#[test]
fn nothing_is_placed_without_a_sighting() {
    let mut hud = chase();
    hud.police_intel.helicopter = Some(HelicopterIntel::new(false, [40.0, 0.0]));
    assert!(threats(&hud, &HudSettings::default()).is_empty(), "no sighting, no place");
    // A helicopter flagged in sight but given no place still places nothing.
    hud.police_intel.helicopter = Some(HelicopterIntel { in_sight: true, position: None });
    assert!(threats(&hud, &HudSettings::default()).is_empty());
}

#[test]
fn confirmed_threats_are_capped_at_three_nearest_first() {
    let mut hud = chase();
    hud.police_intel.sightings = vec![[200.0, 0.0], [50.0, 0.0], [0.0, 120.0], [90.0, 0.0], [0.0, -400.0]];
    hud.police_intel.helicopter = Some(HelicopterIntel::new(true, [0.0, 60.0]));
    let found = threats(&hud, &HudSettings::default());
    assert_eq!(found.len(), MAX_THREATS);
    let distances: Vec<f32> = found.iter().map(|threat| threat.distance).collect();
    assert_eq!(distances, vec![50.0, 60.0, 90.0]);
    assert_eq!(found[1].kind, ThreatKind::Helicopter, "the helicopter in sight is a confirmed threat");
    assert!(found.iter().all(|threat| threat.kind.style() == ThreatStyle::Solid && threat.alpha == 1.0));
}

#[test]
fn the_last_known_place_is_hollow_fades_with_age_and_is_dropped_at_twenty_seconds() {
    assert_eq!(last_known_alpha(0.0), Some(LAST_KNOWN_ALPHA));
    let half = last_known_alpha(LAST_KNOWN_FADE / 2.0).unwrap();
    assert!((half - LAST_KNOWN_ALPHA / 2.0).abs() < 1e-5, "{half}");
    assert!(last_known_alpha(5.0).unwrap() > last_known_alpha(15.0).unwrap(), "older is fainter");
    assert_eq!(last_known_alpha(LAST_KNOWN_FADE), None);
    let mut hud = chase();
    hud.pursuit_phase = PursuitPhase::Search;
    hud.police_intel.last_known = Some(LastKnown { position: [80.0, 0.0], age_secs: 5.0 });
    let found = threats(&hud, &HudSettings::default());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, ThreatKind::LastKnown);
    assert_eq!(found[0].kind.style(), ThreatStyle::Hollow, "drawn apart from a confirmed cop");
    assert!((found[0].alpha - last_known_alpha(5.0).unwrap()).abs() < 1e-6);
    hud.police_intel.last_known = Some(LastKnown { position: [80.0, 0.0], age_secs: 25.0 });
    assert!(threats(&hud, &HudSettings::default()).is_empty(), "faded out");
}

#[test]
fn a_sighting_replaces_the_last_known_place() {
    let mut hud = chase();
    hud.police_intel.last_known = Some(LastKnown { position: [80.0, 0.0], age_secs: 1.0 });
    hud.police_intel.sightings = vec![[30.0, 5.0]];
    let found = threats(&hud, &HudSettings::default());
    assert_eq!(found.iter().map(|threat| threat.kind).collect::<Vec<_>>(), vec![ThreatKind::Cop]);
}

#[test]
fn the_last_known_place_shows_only_while_nobody_sees_the_player() {
    let mut hud = chase();
    hud.police_intel.last_known = Some(LastKnown { position: [80.0, 0.0], age_secs: 1.0 });
    assert!(threats(&hud, &HudSettings::default()).is_empty(), "an active chase sees the player: no last-known mark");
    for phase in [PursuitPhase::Search, PursuitPhase::Cooldown] {
        hud.pursuit_phase = phase;
        let found = threats(&hud, &HudSettings::default());
        assert_eq!(
            found.iter().map(|threat| threat.kind).collect::<Vec<_>>(),
            vec![ThreatKind::LastKnown],
            "{phase:?}"
        );
    }
}

#[test]
fn the_indicators_need_a_chase_and_the_pursuit_info() {
    let mut hud = chase();
    hud.police_intel.sightings = vec![[30.0, 5.0]];
    assert_eq!(threats(&hud, &HudSettings::default()).len(), 1);
    let off = HudSettings { pursuit_info: false, ..HudSettings::default() };
    assert!(threats(&hud, &off).is_empty(), "pursuit_info off");
    hud.pursuit_phase = PursuitPhase::None;
    assert!(threats(&hud, &HudSettings::default()).is_empty(), "no chase");
}

#[test]
fn a_threat_outside_the_view_is_an_edge_arrow_and_one_inside_a_mark() {
    let screen = Screen { width: 1600.0, height: 900.0, pixels_per_point: 1.0 };
    let eye = Vec3::new(0.0, 0.0, 2.0);
    let lens = Lens::look_at(eye, eye + Vec3::X, 60.0, 0.3, 16.0 / 9.0);
    let camera = HudCamera { view_proj: lens.view_proj().to_cols_array_2d(), viewport: [1600.0, 900.0] };
    let mut hud = chase();
    // One ahead, one behind and to the left.
    hud.police_intel.sightings = vec![[60.0, 0.0], [-40.0, 30.0]];
    let window = window(screen);
    let placed = place(threats(&hud, &HudSettings::default()), &camera, window, edge_rect(window, 9.0), 7.0);
    assert_eq!(placed.len(), 2);
    let behind = placed.iter().find(|p| p.threat.position[0] < 0.0).unwrap();
    let arrow = behind.arrow.expect("behind the camera: an edge arrow");
    assert!(arrow.x < 0.0, "it points left: {arrow:?}");
    let ahead = placed.iter().find(|p| p.threat.position[0] > 0.0).unwrap();
    assert!(ahead.arrow.is_none(), "in view: a mark on its place");
}
