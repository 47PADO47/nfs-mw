//! Tests for which markers show and how (`rules.rs`): the candidates, the styling bands, occlusion, the priority order
//! and the cap.

use nfsmw_data::gameplay::RaceType;

use super::rules::{
    BREAKER_RANGE, Candidate, EMPHASIS_SIZE, ENTRANCE_RANGE, FAR_ALPHA, FAR_SIZE, GATE_LIFT, GATE_RANGE, Kind,
    MAX_MARKERS, Name, OCCLUDED_ALPHA, PLACE_LIFT, gather, look, select,
};
use super::tests::{car_at_origin, entrance, race_to};
use crate::hud::{MarkPoint, NavMarkerKind};
use crate::settings::hud::HudSettings;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn marks_are_full_near_and_fade_and_shrink_to_their_range() {
    let near = look(Kind::Entrance, 30.0, false).unwrap();
    assert_eq!((near.alpha, near.size, near.label), (1.0, 1.0, true));
    let far = look(Kind::Entrance, ENTRANCE_RANGE, false).unwrap();
    assert!(close(far.alpha, FAR_ALPHA) && close(far.size, FAR_SIZE), "{far:?}");
    let mid = look(Kind::Breaker, (50.0 + BREAKER_RANGE) / 2.0, false).unwrap();
    assert!(close(mid.alpha, (1.0 + FAR_ALPHA) / 2.0) && close(mid.size, (1.0 + FAR_SIZE) / 2.0), "{mid:?}");
    assert!(look(Kind::Entrance, ENTRANCE_RANGE + 1.0, false).is_none(), "past its range a place is gone");
    assert!(look(Kind::Breaker, BREAKER_RANGE + 1.0, false).is_none());
}

#[test]
fn the_gate_stays_past_its_range_dimmed_and_without_a_label_past_300_m() {
    let far = look(Kind::ActiveCheckpoint, GATE_RANGE * 2.0, false).unwrap();
    assert!(close(far.alpha, FAR_ALPHA) && close(far.size, FAR_SIZE), "{far:?}");
    assert!(!far.label, "no label past 300 m");
    assert!(look(Kind::ActiveCheckpoint, 250.0, false).unwrap().label);
    assert!(look(Kind::Recovery, GATE_RANGE * 2.0, false).is_some());
}

#[test]
fn the_active_checkpoint_is_emphasised_as_the_player_nears() {
    let near = look(Kind::ActiveCheckpoint, 120.0, false).unwrap();
    let plain = look(Kind::Recovery, 120.0, false).unwrap();
    assert_eq!(near.alpha, 1.0, "full alpha within 150 m");
    assert!(close(near.size, plain.size * EMPHASIS_SIZE), "{near:?} against {plain:?}");
    let before = look(Kind::ActiveCheckpoint, 200.0, false).unwrap();
    assert!(before.alpha < 1.0 && before.size < near.size, "not yet emphasised: {before:?}");
}

#[test]
fn geometry_dims_a_checkpoint_and_hides_the_rest() {
    let gate = look(Kind::ActiveCheckpoint, 80.0, true).unwrap();
    assert_eq!((gate.alpha, gate.label), (OCCLUDED_ALPHA, false), "checkpoints keep guidance through walls");
    assert!(look(Kind::Recovery, 80.0, true).is_some());
    assert!(look(Kind::Entrance, 80.0, true).is_none(), "event entrances hide");
    assert!(look(Kind::Breaker, 80.0, true).is_none(), "a hidden breaker is not in sight");
    assert!(look(Kind::Discovered, 80.0, true).is_none());
}

#[test]
fn breakers_and_discovered_places_are_icons_only() {
    assert!(!look(Kind::Breaker, 20.0, false).unwrap().label);
    assert!(!look(Kind::Discovered, 20.0, false).unwrap().label);
}

fn candidate(kind: Kind, distance: f32) -> Candidate {
    Candidate { id: distance as u64, kind, position: [0.0; 3], distance, occluded: false, name: Name::None }
}

#[test]
fn the_cap_keeps_the_priority_order_and_the_nearest_within_a_kind() {
    let mut items = vec![
        candidate(Kind::Discovered, 5.0),
        candidate(Kind::Entrance, 90.0),
        candidate(Kind::Entrance, 40.0),
        candidate(Kind::Breaker, 120.0),
        candidate(Kind::Entrance, 10.0),
        candidate(Kind::Recovery, 300.0),
        candidate(Kind::Breaker, 60.0),
        candidate(Kind::Entrance, 200.0),
    ];
    select(&mut items, MAX_MARKERS, |item| (item.kind, item.distance));
    let kept: Vec<(Kind, f32)> = items.iter().map(|item| (item.kind, item.distance)).collect();
    assert_eq!(
        kept,
        vec![
            (Kind::Recovery, 300.0),
            (Kind::Breaker, 60.0),
            (Kind::Breaker, 120.0),
            (Kind::Entrance, 10.0),
            (Kind::Entrance, 40.0),
            (Kind::Entrance, 90.0),
        ],
        "at most six: the nearest discovered place loses to every higher kind"
    );
}

#[test]
fn the_priority_order_is_the_specs() {
    let order = [Kind::ActiveCheckpoint, Kind::Recovery, Kind::Breaker, Kind::Entrance, Kind::Discovered];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn the_next_gate_is_the_active_checkpoint_ahead_and_a_recovery_behind_or_after_a_wrong_way() {
    let settings = HudSettings::default();
    let ahead = gather(&race_to([100.0, 0.0, 1.0]), &settings, false);
    assert_eq!(ahead.len(), 1);
    assert_eq!(ahead[0].kind, Kind::ActiveCheckpoint);
    assert_eq!(ahead[0].name, Name::Gate { gate: 3, gates: 12 });
    assert_eq!(ahead[0].position, [100.0, 0.0, 1.0 + GATE_LIFT]);
    assert!(close(ahead[0].distance, 100.0));
    let behind = gather(&race_to([-100.0, 0.0, 1.0]), &settings, false);
    assert_eq!(behind[0].kind, Kind::Recovery, "the gate is behind the car");
    assert_eq!(behind[0].id, ahead[0].id, "one id for the gate whichever way it draws");
    let passing = gather(&race_to([-10.0, 0.0, 1.0]), &settings, false);
    assert_eq!(passing[0].kind, Kind::ActiveCheckpoint, "a gate just behind is being driven through");
    let wrong_way = gather(&race_to([100.0, 0.0, 1.0]), &settings, true);
    assert_eq!(wrong_way[0].kind, Kind::Recovery);
}

#[test]
fn places_become_entrances_and_with_the_extras_discovered_places() {
    let mut hud = car_at_origin();
    hud.markers = vec![
        entrance(NavMarkerKind::Race(RaceType::Sprint), [50.0, 0.0], Some("10.2.1")),
        entrance(NavMarkerKind::SafeHouse, [0.0, 80.0], None),
    ];
    hud.world_marks.entrances = vec![
        crate::hud::EntranceMark { marker: 0, point: MarkPoint { position: [50.0, 0.0, 4.0], occluded: false } },
        crate::hud::EntranceMark { marker: 1, point: MarkPoint { position: [0.0, 80.0, 2.0], occluded: true } },
        crate::hud::EntranceMark { marker: 7, point: MarkPoint { position: [0.0, 9.0, 2.0], occluded: false } },
    ];
    let plain = gather(&hud, &HudSettings::default(), false);
    assert_eq!(plain.len(), 1, "a discovered place needs the free-roam extras, and a stale index is skipped");
    assert_eq!(plain[0].kind, Kind::Entrance);
    assert_eq!(plain[0].position, [50.0, 0.0, 4.0 + PLACE_LIFT]);
    assert_eq!(plain[0].name, Name::Place(NavMarkerKind::Race(RaceType::Sprint), Some("10.2.1".to_string())));
    let extras = HudSettings { free_roam_extras: true, ..HudSettings::default() };
    let all = gather(&hud, &extras, false);
    assert_eq!(all.iter().map(|c| c.kind).collect::<Vec<_>>(), vec![Kind::Entrance, Kind::Discovered]);
    assert!(all[1].occluded, "the scene's occlusion is carried");
}

#[test]
fn breakers_in_sight_are_candidates_and_nothing_shows_without_a_car() {
    let mut hud = car_at_origin();
    hud.world_marks.breakers = vec![MarkPoint { position: [30.0, 40.0, 0.0], occluded: false }];
    let breakers = gather(&hud, &HudSettings::default(), false);
    assert_eq!(breakers.len(), 1);
    assert_eq!(breakers[0].kind, Kind::Breaker);
    assert!(close(breakers[0].distance, 50.0));
    hud.minimap = None;
    assert!(gather(&hud, &HudSettings::default(), false).is_empty());
}
