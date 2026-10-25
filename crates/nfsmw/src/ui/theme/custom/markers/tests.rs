//! Tests for the world markers as a whole (`mod.rs`, `memory.rs`, `draw.rs`): placing and capping, the memory's events
//! and label steadying, the labels' text, and the component drawing or not drawing.

use std::sync::Arc;

use blackbox_minimap::Orientation;
use blackbox_ui::{Component, DrawList, LayoutCx, Rect, Skin};
use glam::{Vec2, Vec3};
use nfsmw_data::gameplay::RaceType;

use super::draw::{ARROW_HALF, distance_text};
use super::memory::{MarkerMemory, RECOVERY_SECS};
use super::project::edge_rect;
use super::rules::{Candidate, Kind, MAX_MARKERS, Name};
use super::{Placed, WorldMarkers, arrange, label_target};
use crate::hud::{HudCamera, HudEvent, HudState, MapPosition, NavMarker, NavMarkerKind, RaceInfo};
use crate::settings::SpeedUnits;
use crate::settings::hud::{HudPalette, HudSettings};
use crate::ui::present::Screen;
use crate::ui::theme::custom::feed::{CustomFeed, CustomFeedCell};
use crate::ui::theme::custom::layout::window;
use crate::ui::theme::custom::tokens::{CustomLook, skin};
use crate::viewer::camera::Lens;

const SCREEN: Screen = Screen { width: 1600.0, height: 900.0, pixels_per_point: 1.0 };

/// A HUD state with the car at the origin heading along +x, and nothing else.
pub(super) fn car_at_origin() -> HudState {
    let minimap = MapPosition { position: [0.0, 0.0], heading: [1.0, 0.0], orientation: Orientation::North };
    HudState { minimap: Some(minimap), ..HudState::default() }
}

/// The car at the origin in a race whose next gate, gate 3 of 12, is at `gate`.
pub(super) fn race_to(gate: [f32; 3]) -> HudState {
    let race = RaceInfo {
        kind: RaceType::Sprint,
        next_gate: Some(gate),
        gate: Some(3),
        gates: 12,
        remaining_m: None,
        rival_gap: None,
    };
    HudState { race_info: Some(race), ..car_at_origin() }
}

/// A map place of `kind` at `position`.
pub(super) fn entrance(kind: NavMarkerKind, position: [f32; 2], name: Option<&str>) -> NavMarker {
    NavMarker { kind, position, name: name.map(str::to_string), discovered: true }
}

/// The game's lens two metres up at the origin, looking along +x, over a 1600 x 900 window.
fn camera() -> HudCamera {
    let eye = Vec3::new(0.0, 0.0, 2.0);
    let lens = Lens::look_at(eye, eye + Vec3::X, 60.0, 0.3, 16.0 / 9.0);
    HudCamera { view_proj: lens.view_proj().to_cols_array_2d(), viewport: [SCREEN.width, SCREEN.height] }
}

fn candidate(kind: Kind, position: [f32; 3]) -> Candidate {
    let distance = position[0].hypot(position[1]);
    Candidate { id: position[1].to_bits().into(), kind, position, distance, occluded: false, name: Name::None }
}

#[test]
fn a_gate_behind_the_camera_becomes_an_edge_arrow_and_a_place_behind_it_is_dropped() {
    let window = window(SCREEN);
    let candidates = vec![candidate(Kind::Recovery, [-80.0, 20.0, 1.0]), candidate(Kind::Entrance, [-50.0, 0.0, 1.0])];
    let placed = arrange(candidates, &camera(), window, 1.0);
    assert_eq!(placed.len(), 1, "the entrance behind the camera has no guidance to give");
    let arrow = placed[0].arrow.expect("an edge arrow");
    assert!(arrow.x < 0.0, "behind and to the left points left: {arrow:?}");
    let edge = edge_rect(window, ARROW_HALF);
    let on = (placed[0].at.x / (edge.w / 2.0)).powi(2) + (placed[0].at.y / (edge.h / 2.0)).powi(2);
    assert!((on - 1.0).abs() < 1e-3, "on the edge ellipse: {:?}", placed[0].at);
    assert!(placed[0].look.label, "an edge arrow names the distance");
}

#[test]
fn no_more_than_the_cap_are_placed_in_priority_order() {
    let mut candidates: Vec<Candidate> =
        (0..8).map(|i| candidate(Kind::Entrance, [60.0 + i as f32 * 10.0, i as f32, 1.0])).collect();
    candidates.push(candidate(Kind::ActiveCheckpoint, [250.0, -3.0, 1.0]));
    let placed = arrange(candidates, &camera(), window(SCREEN), 1.0);
    assert_eq!(placed.len(), MAX_MARKERS);
    assert_eq!(placed[0].candidate.kind, Kind::ActiveCheckpoint, "the gate first, though farthest");
    let distances: Vec<f32> = placed[1..].iter().map(|p| p.candidate.distance).collect();
    assert!(distances.windows(2).all(|pair| pair[0] <= pair[1]), "then the nearest entrances: {distances:?}");
    assert!(placed.iter().all(|p| p.arrow.is_none()), "all in view");
}

#[test]
fn labels_sit_under_a_mark_and_inward_of_an_arrow() {
    let mark = Placed {
        candidate: candidate(Kind::ActiveCheckpoint, [100.0, 0.0, 1.0]),
        look: super::rules::Look { alpha: 1.0, size: 1.0, label: true },
        at: Vec2::new(10.0, 20.0),
        arrow: None,
    };
    let under = label_target(&mark, Vec2::new(40.0, 14.0), 1.0);
    assert!(under.x == 10.0 && under.y > 20.0, "{under:?}");
    let arrow = Placed { arrow: Some(Vec2::X), at: Vec2::new(400.0, 0.0), ..mark };
    let inward = label_target(&arrow, Vec2::new(40.0, 14.0), 1.0);
    assert!(inward.x < 400.0 - ARROW_HALF - 20.0 && inward.y == 0.0, "{inward:?}");
}

fn frame(events_frame: u64, events: Vec<HudEvent>, gate: Option<[f32; 3]>) -> HudState {
    let mut hud = gate.map_or_else(car_at_origin, race_to);
    hud.events = events;
    hud.events_frame = events_frame;
    hud
}

#[test]
fn a_checkpoint_pass_ticks_at_the_gate_just_passed_once() {
    let mut memory = MarkerMemory::default();
    let first = [100.0, 0.0, 1.0];
    memory.observe(&frame(1, vec![], Some(first)), 1.0);
    // The pass: the race already points at the next gate.
    let pass = frame(2, vec![HudEvent::Checkpoint { number: 1, total: 12 }], Some([300.0, 0.0, 1.0]));
    memory.observe(&pass, 2.0);
    let (tick, progress) = memory.tick(2.1, 0.4).expect("the tick runs");
    assert_eq!(tick.at, first, "at the gate that was passed");
    assert!((progress - 0.25).abs() < 1e-4);
    assert!(memory.tick(2.5, 0.4).is_none(), "brief: over after the pulse");
    // A paused scene hands over the same frame again: its events are not taken twice.
    memory.observe(&pass, 3.0);
    assert!(memory.tick(3.1, 0.4).is_none());
}

#[test]
fn a_wrong_way_starts_the_recovery_for_a_while_and_leaving_the_race_ends_it() {
    let mut memory = MarkerMemory::default();
    memory.observe(&frame(1, vec![HudEvent::WrongWay], Some([100.0, 0.0, 1.0])), 10.0);
    assert!(memory.recovering(10.0) && memory.recovering(10.0 + RECOVERY_SECS - 0.1));
    assert!(!memory.recovering(10.0 + RECOVERY_SECS));
    memory.observe(&frame(2, vec![HudEvent::WrongWay], Some([100.0, 0.0, 1.0])), 20.0);
    memory.observe(&frame(3, vec![], None), 20.5);
    assert!(!memory.recovering(20.6), "no race, no recovery");
}

#[test]
fn the_memory_steadies_labels_per_marker_and_forgets_the_gone() {
    let mut memory = MarkerMemory::default();
    let dt = memory.observe(&car_at_origin(), 1.0);
    assert_eq!(dt, 0.0, "the first frame has no time before it");
    assert_eq!(memory.label(7, Vec2::new(10.0, 0.0), dt), Vec2::new(10.0, 0.0), "a new label starts on its target");
    let dt = memory.observe(&car_at_origin(), 1.0 + 1.0 / 60.0);
    let shaken = memory.label(7, Vec2::new(14.0, 0.0), dt);
    assert!(shaken.x > 10.0 && shaken.x < 12.0, "a shake moves it only part way: {shaken:?}");
    assert_eq!(memory.observe(&car_at_origin(), 1.0 + 1.0 / 60.0), 0.0, "a second layout in the frame takes no time");
    memory.keep_labels(&[]);
    assert_eq!(memory.label(7, Vec2::new(14.0, 0.0), 0.1), Vec2::new(14.0, 0.0), "forgotten, it starts afresh");
}

#[test]
fn distances_read_in_the_players_units_in_steady_steps() {
    assert_eq!(distance_text(5.4, SpeedUnits::Metric), "5 m");
    assert_eq!(distance_text(123.0, SpeedUnits::Metric), "120 m");
    assert_eq!(distance_text(1500.0, SpeedUnits::Metric), "1.5 km");
    assert_eq!(distance_text(100.0, SpeedUnits::Imperial), "330 ft");
    assert_eq!(distance_text(1000.0, SpeedUnits::Imperial), "0.6 mi");
}

fn token_skin(high_contrast: bool) -> Skin {
    skin(CustomLook { palette: HudPalette::Default, high_contrast }, [255, 200, 80, 255])
}

/// The nodes the markers draw for `hud` with `settings`.
fn drawn(hud: HudState, settings: HudSettings, high_contrast: bool) -> usize {
    let cell = CustomFeedCell::new();
    let hud = HudState { camera: Some(camera()), ..hud };
    cell.set(CustomFeed { hud: Arc::new(hud), settings, screen: SCREEN, elapsed_s: 1.0, ..CustomFeed::default() });
    let skin = token_skin(high_contrast);
    let mut markers = WorldMarkers::new(cell);
    markers.layout(&LayoutCx::new(&skin, 1.0), Rect::SCREEN);
    let mut out = DrawList::new();
    markers.build(&mut out);
    out.len()
}

#[test]
fn the_markers_draw_the_next_gate_and_respect_the_setting_and_a_hidden_hud() {
    let race = race_to([120.0, 0.0, 0.0]);
    let shown = drawn(race.clone(), HudSettings::default(), false);
    assert!(shown > 0, "the gate and its label draw");
    let off = HudSettings { world_markers: false, ..HudSettings::default() };
    assert_eq!(drawn(race.clone(), off, false), 0, "world_markers off draws nothing");
    assert_eq!(drawn(HudState { visible: false, ..race.clone() }, HudSettings::default(), false), 0);
    assert!(drawn(race, HudSettings::default(), true) > shown, "high contrast adds the outlines");
    assert_eq!(drawn(car_at_origin(), HudSettings::default(), false), 0, "nothing to mark, nothing drawn");
}
