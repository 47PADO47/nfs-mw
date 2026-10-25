//! Tests for the map's logic, with no drawing: the mode cycle and the keys, the mode change and the zoom spring, the
//! local set's cache rule, fair knowledge in the marks, and the chip's distances.

use crate::hud::{
    CHECKPOINT, COP, HELICOPTER, HelicopterIntel, HudState, LastKnown, MapBlip, MapRoad, MapRoads, MapRoadsHandle,
    NavMarker, NavMarkerKind, PoliceIntel, PursuitPhase, RACER, RaceInfo, RoadClass,
};
use crate::settings::hud::MapMode;
use crate::settings::{MinimapMode, Partial, Settings, SpeedUnits};

use super::chip::distance_text;
use super::draw::split_route;
use super::marks::{CAR_LOT, LAST_KNOWN_ALPHA, MarkKind, SHOP, last_known_alpha, marks};
use super::memory::{ModeFade, ZoomSpring};
use super::modes::{
    CENTERED_RADII, COMPACT_RADII, EXPANDED_RADII, MapKeys, MapPresses, Overlay, expanded_opacity, flip_orientation,
    next_mode, transition,
};
use super::roads::{LocalSet, MOVE_SHARE};

fn settings() -> Settings {
    Partial::default().into()
}

#[test]
fn the_mode_key_cycles_off_compact_centered_expanded() {
    let mut mode = MapMode::Off;
    let mut seen = Vec::new();
    for _ in 0..4 {
        mode = next_mode(mode);
        seen.push(mode);
    }
    assert_eq!(seen, [MapMode::Compact, MapMode::Centered, MapMode::Expanded, MapMode::Off]);
}

#[test]
fn expand_opens_the_expanded_map_and_goes_back_to_where_it_came_from() {
    let mut s = settings();
    s.hud_options.map_mode = MapMode::Compact;
    let mut keys = MapKeys::default();
    let expand = MapPresses { expand: true, ..Default::default() };
    keys.apply(expand, &mut s);
    assert_eq!(s.hud_options.map_mode, MapMode::Expanded);
    keys.apply(expand, &mut s);
    assert_eq!(s.hud_options.map_mode, MapMode::Compact, "back to the mode it was opened from");
    // Reached by the cycle, the way back is Centered.
    s.hud_options.map_mode = MapMode::Expanded;
    keys.apply(expand, &mut s);
    assert_eq!(s.hud_options.map_mode, MapMode::Centered);
}

#[test]
fn the_keys_zoom_within_the_range_turn_the_map_and_hide_the_hud() {
    let mut s = settings();
    let mut keys = MapKeys::default();
    let zoom_in = MapPresses { zoom_in: true, ..Default::default() };
    keys.apply(zoom_in, &mut s);
    assert!((s.hud_options.map_zoom - 1.25).abs() < 1e-5);
    for _ in 0..10 {
        keys.apply(zoom_in, &mut s);
    }
    assert_eq!(s.hud_options.map_zoom, 2.0, "clamped to the range");
    for _ in 0..20 {
        keys.apply(MapPresses { zoom_out: true, ..Default::default() }, &mut s);
    }
    assert_eq!(s.hud_options.map_zoom, 0.5);
    assert_eq!(s.minimap, MinimapMode::Fixed);
    keys.apply(MapPresses { orientation: true, ..Default::default() }, &mut s);
    assert_eq!(s.minimap, MinimapMode::Rotating);
    assert_eq!(flip_orientation(MinimapMode::Rotating), MinimapMode::Fixed);
    assert_eq!(flip_orientation(MinimapMode::Off), MinimapMode::Rotating);
    let shown = s.hud;
    keys.apply(MapPresses { toggle_hud: true, ..Default::default() }, &mut s);
    assert_eq!(s.hud, !shown);
    assert!(!MapPresses::default().any());
}

#[test]
fn expanded_is_larger_and_more_opaque_and_off_shows_nothing() {
    let centered = Overlay::of(MapMode::Centered, 0.35);
    let expanded = Overlay::of(MapMode::Expanded, 0.35);
    assert_eq!((centered.radii, expanded.radii), (CENTERED_RADII, EXPANDED_RADII));
    assert!(expanded.base_m > centered.base_m);
    assert!((expanded.road_opacity - 0.6).abs() < 1e-5);
    assert_eq!(expanded_opacity(0.9), 0.9, "never below the setting");
    assert_eq!(expanded_opacity(0.7), 0.85, "capped");
    assert_eq!((Overlay::of(MapMode::Off, 0.35).shown, Overlay::of(MapMode::Off, 0.35).chip), (0.0, 0.0));
}

#[test]
fn compact_is_a_small_faint_overlay_with_the_turn_chip() {
    let (compact, centered) = (Overlay::of(MapMode::Compact, 0.35), Overlay::of(MapMode::Centered, 0.35));
    assert_eq!((compact.shown, compact.chip, compact.radii), (1.0, 1.0, COMPACT_RADII));
    assert!(compact.radii[0] < centered.radii[0] && compact.base_m < centered.base_m);
    assert!(compact.road_opacity < centered.road_opacity, "fainter than Centered");
}

#[test]
fn the_overlays_are_the_widths_the_design_asks_for_at_1080p() {
    // The box is 480 units high for 1080 pixels: 2.25 pixels a unit, and the ellipse's width is twice its half-axis.
    let width = |radii: [f32; 2]| 2.0 * radii[0] * 480.0 * 2.25;
    assert!((300.0..=380.0).contains(&width(COMPACT_RADII)), "{}", width(COMPACT_RADII));
    assert!((430.0..=560.0).contains(&width(CENTERED_RADII)), "{}", width(CENTERED_RADII));
    assert!((620.0..=760.0).contains(&width(EXPANDED_RADII)), "{}", width(EXPANDED_RADII));
}

#[test]
fn a_mode_change_glides_with_full_motion_and_only_fades_with_reduced_motion() {
    let (from, to) = (Overlay::of(MapMode::Centered, 0.35), Overlay::of(MapMode::Expanded, 0.35));
    let half = transition(from, to, 0.5, false);
    assert!(half.radii[0] > from.radii[0] && half.radii[0] < to.radii[0]);
    let reduced = transition(from, to, 0.5, true);
    assert_eq!(reduced.radii, to.radii, "the shape jumps");
    assert_eq!(reduced.base_m, to.base_m);
    let off = transition(from, Overlay::of(MapMode::Off, 0.35), 0.5, true);
    assert!(off.shown > 0.0 && off.shown < 1.0, "the overlay fades out");
    let done = transition(from, to, 1.0, false);
    assert!((done.radii[0] - to.radii[0]).abs() < 1e-5 && (done.base_m - to.base_m).abs() < 1e-3);
    assert_eq!((done.shown, done.detail), (1.0, 1.0));
}

#[test]
fn the_mode_fade_starts_settled_and_times_a_change() {
    let mut fade = ModeFade::default();
    fade.update(MapMode::Centered, 5.0);
    assert_eq!((fade.previous(), fade.current()), (MapMode::Centered, MapMode::Centered));
    assert_eq!(fade.progress(5.0, 0.3), 1.0, "the first frame is settled");
    fade.update(MapMode::Expanded, 10.0);
    assert_eq!((fade.previous(), fade.current()), (MapMode::Centered, MapMode::Expanded));
    assert!((fade.progress(10.15, 0.3) - 0.5).abs() < 1e-4);
    fade.update(MapMode::Expanded, 10.2);
    assert_eq!(fade.progress(10.4, 0.3), 1.0, "the same mode does not restart the change");
}

#[test]
fn the_zoom_glides_to_its_target_or_jumps_with_reduced_motion() {
    let mut spring = ZoomSpring::default();
    assert_eq!(spring.step(1.0, 0.016, 0.0, 0.25, false), 1.0, "the first frame takes the target");
    let mut zoom = 1.0;
    let mut t = 0.0;
    for _ in 0..3 {
        t += 0.016;
        zoom = spring.step(2.0, 0.016, t, 0.25, false);
    }
    assert!(zoom > 1.0 && zoom < 2.0, "still gliding: {zoom}");
    for _ in 0..60 {
        t += 0.016;
        zoom = spring.step(2.0, 0.016, t, 0.25, false);
    }
    assert!((zoom - 2.0).abs() < 0.01, "settled: {zoom}");
    assert_eq!(spring.crossfade(t, 0.15, false), 1.0);
    let mut reduced = ZoomSpring::default();
    reduced.step(1.0, 0.016, 0.0, 0.15, true);
    assert_eq!(reduced.step(0.5, 0.016, 1.0, 0.15, true), 0.5, "jumps");
    assert_eq!(reduced.crossfade(1.0, 0.15, true), 0.0, "and fades back in");
    assert_eq!(reduced.crossfade(1.2, 0.15, true), 1.0);
}

fn roads() -> MapRoadsHandle {
    let road = |points: Vec<[f32; 2]>| MapRoad { class: RoadClass::Main, points };
    MapRoadsHandle::new(MapRoads::from_roads(vec![
        road(vec![[0.0, 0.0], [100.0, 0.0]]),
        road(vec![[5000.0, 0.0], [5100.0, 0.0]]),
    ]))
}

#[test]
fn the_local_set_is_queried_again_only_past_the_move_or_zoom_threshold() {
    let net = roads();
    let mut local = LocalSet::default();
    assert!(local.refresh(&net, [0.0, 0.0], 300.0), "the first frame queries");
    assert_eq!(local.lines(), [0]);
    for step in 1..=10 {
        let x = step as f32 * 7.0;
        assert!(!local.refresh(&net, [x, 0.0], 300.0), "{x} m is below the threshold");
    }
    assert!(!local.refresh(&net, [0.0, 0.0], 320.0), "a zoom change under 10 %");
    assert!(local.refresh(&net, [MOVE_SHARE * 300.0 + 1.0, 0.0], 300.0), "past the move threshold");
    assert!(local.refresh(&net, [MOVE_SHARE * 300.0 + 1.0, 0.0], 400.0), "past the zoom threshold");
    assert!(!local.refresh(&net, [MOVE_SHARE * 300.0 + 1.0, 0.0], 400.0), "an unchanged frame");
    assert!(local.refresh(&roads(), [MOVE_SHARE * 300.0 + 1.0, 0.0], 400.0), "another network");
    assert!(local.refresh(&net, [5050.0, 0.0], 300.0));
    assert_eq!(local.lines(), [1]);
}

fn chase(phase: PursuitPhase, police: PoliceIntel) -> HudState {
    HudState { pursuit_phase: phase, police_intel: police, ..HudState::default() }
}

fn kinds(hud: &HudState) -> Vec<MarkKind> {
    marks(hud).iter().map(|mark| mark.kind).collect()
}

#[test]
fn no_cop_is_marked_without_a_sighting() {
    // Cop and helicopter blips follow the original minimap's rules: the Custom map never reads them.
    let blips = vec![
        MapBlip { style: COP, position: [10.0, 0.0], active: true, heading: None },
        MapBlip { style: HELICOPTER, position: [20.0, 0.0], active: true, heading: None },
    ];
    let unseen = PoliceIntel { helicopter: Some(HelicopterIntel::new(false, [20.0, 0.0])), ..Default::default() };
    let hud = HudState { blips, ..chase(PursuitPhase::Search, unseen) };
    assert!(kinds(&hud).is_empty(), "{:?}", kinds(&hud));
    let seen = PoliceIntel {
        sightings: vec![[10.0, 0.0]],
        helicopter: Some(HelicopterIntel::new(true, [20.0, 0.0])),
        ..Default::default()
    };
    assert_eq!(kinds(&chase(PursuitPhase::Active, seen)), [MarkKind::Helicopter, MarkKind::Cop]);
}

#[test]
fn the_last_known_place_fades_with_age_and_shows_only_while_unseen() {
    let last = |age_secs| LastKnown { position: [1.0, 2.0], age_secs };
    assert_eq!(last_known_alpha(PursuitPhase::Search, last(0.0)), Some(LAST_KNOWN_ALPHA));
    let half = last_known_alpha(PursuitPhase::Cooldown, last(10.0)).unwrap();
    assert!((half - LAST_KNOWN_ALPHA / 2.0).abs() < 1e-5);
    assert!(last_known_alpha(PursuitPhase::Search, last(15.0)).unwrap() < half, "older is fainter");
    assert_eq!(last_known_alpha(PursuitPhase::Search, last(20.0)), None, "gone at 20 s");
    assert_eq!(last_known_alpha(PursuitPhase::Active, last(1.0)), None, "seen now: no last-known mark");
    assert_eq!(last_known_alpha(PursuitPhase::None, last(1.0)), None);
    let hud = chase(PursuitPhase::Search, PoliceIntel { last_known: Some(last(5.0)), ..Default::default() });
    let mark = marks(&hud)[0];
    assert_eq!((mark.kind, mark.at), (MarkKind::LastKnown, [1.0, 2.0]));
}

#[test]
fn a_roadblock_is_marked_only_once_reported() {
    let sites = vec![[3.0, 3.0]];
    let unreported = PoliceIntel { roadblocks: sites.clone(), ..Default::default() };
    assert!(kinds(&chase(PursuitPhase::Active, unreported)).is_empty());
    let reported = PoliceIntel { roadblocks: sites, roadblock_reported: true, ..Default::default() };
    assert_eq!(kinds(&chase(PursuitPhase::Active, reported)), [MarkKind::Roadblock]);
}

#[test]
fn places_and_the_race_are_marked_and_the_next_gate_once() {
    let place = |kind, discovered| NavMarker { kind, position: [0.0, 0.0], name: None, discovered };
    let markers = vec![place(NavMarkerKind::SafeHouse, true), place(NavMarkerKind::AutoShop, false)];
    let race = RaceInfo {
        kind: nfsmw_data::gameplay::RaceType::Sprint,
        next_gate: Some([50.0, 60.0, 1.0]),
        gate: Some(1),
        gates: 3,
        remaining_m: None,
        rival_gap: None,
    };
    let blips = vec![MapBlip { style: CHECKPOINT, position: [50.5, 60.0], active: true, heading: None }];
    let hud = HudState { markers, race_info: Some(race), blips, ..HudState::default() };
    assert_eq!(kinds(&hud), [MarkKind::SafeHouse, MarkKind::NextCheckpoint], "an undiscovered shop is not shown");
}

#[test]
fn the_route_splits_where_the_car_is() {
    let path = [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]];
    let (behind, ahead) = split_route(&path, [40.0, 5.0]);
    assert_eq!(behind, vec![[0.0, 0.0], [40.0, 0.0]]);
    assert_eq!(ahead, vec![[40.0, 0.0], [100.0, 0.0], [100.0, 100.0]]);
    assert_eq!(split_route(&[[1.0, 1.0]], [0.0, 0.0]), (Vec::new(), vec![[1.0, 1.0]]));
}

#[test]
fn the_chip_rounds_the_distance_in_the_players_unit() {
    assert_eq!(distance_text(347.0, SpeedUnits::Metric), "350 m");
    assert_eq!(distance_text(-3.0, SpeedUnits::Metric), "0 m");
    assert_eq!(distance_text(100.0, SpeedUnits::Imperial), "350 ft");
    assert_eq!(distance_text(1609.344, SpeedUnits::Imperial), "1.0 mi");
}

#[test]
fn a_mark_carries_the_variant_and_heading_of_the_games_icon() {
    let place = |kind| NavMarker { kind, position: [1.0, 2.0], name: None, discovered: true };
    let markers = vec![
        place(NavMarkerKind::Race(nfsmw_data::gameplay::RaceType::Drag)),
        place(NavMarkerKind::CarLot),
        place(NavMarkerKind::AutoShop),
    ];
    let blips = vec![
        MapBlip { style: RACER, position: [5.0, 0.0], active: false, heading: Some([0.0, 1.0]) },
        MapBlip { style: RACER, position: [6.0, 0.0], active: false, heading: None },
    ];
    let marks = marks(&HudState { markers, blips, ..HudState::default() });
    let variants: Vec<(MarkKind, u8)> = marks.iter().map(|mark| (mark.kind, mark.variant)).collect();
    let expected = [(MarkKind::Race, 2), (MarkKind::Shop, CAR_LOT), (MarkKind::Shop, SHOP)];
    assert_eq!(variants, [expected[0], expected[1], expected[2], (MarkKind::Racer, 0), (MarkKind::Racer, 1)]);
    assert_eq!((marks[3].heading, marks[4].heading), (Some([0.0, 1.0]), None));
}
