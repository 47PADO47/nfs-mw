use super::lobby::{lobby_state, row};
use super::*;
use crate::message::{
    HeatChange, LeaveReason, List, PhaseChange, PlayerLeft, RaceResults, RaceStart, ResultRow, Role, RoleChanged,
    SessionPhase,
};

fn heat(head: EventHead) -> Message {
    HeatChange { head, entity_id: ENTITY, level: 2 }.into()
}

fn game_events(events: &[SessionEvent]) -> Vec<&Message> {
    events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::Game(message) => Some(message),
            _ => None,
        })
        .collect()
}

#[test]
fn duplicate_and_old_sequence_numbers_are_dropped() {
    let mut h = Harness::synced();
    h.take_events();
    h.reliable(heat(head(1, 600, 0)));
    h.reliable(heat(head(1, 600, 0)));
    h.reliable(heat(head(3, 601, 0)));
    h.reliable(heat(head(2, 601, 0)));
    h.frame();
    let events = h.take_events();
    assert_eq!(game_events(&events).len(), 2);
}

#[test]
fn events_with_a_target_tick_wait_for_it() {
    let mut h = Harness::synced();
    h.take_events();
    let now_tick = server_tick(h.now) as u32;
    let target = now_tick + 30;
    let phase: Message = PhaseChange { head: head(1, now_tick, target), phase: SessionPhase::Running }.into();
    h.reliable(phase.clone());
    h.frame();
    assert!(game_events(&h.take_events()).is_empty(), "held until the target tick");
    h.run(600);
    h.answer_pings();
    let events = h.take_events();
    assert_eq!(game_events(&events), vec![&phase]);
    assert!(events.contains(&SessionEvent::LobbyChanged));
    assert_eq!(h.session.lobby().phase, SessionPhase::Running);
}

#[test]
fn a_race_start_fixes_the_start_tick_at_once() {
    let mut h = Harness::synced();
    let target = server_tick(h.now) as u32 + 300;
    let order = List::new(vec![ENTITY, 7]).unwrap();
    h.reliable(RaceStart { head: head(1, 0, target), order }.into());
    h.frame();
    assert_eq!(h.session.lobby().start_tick, Some(target));
}

#[test]
fn results_are_kept_and_reported() {
    let mut h = Harness::synced();
    h.take_events();
    let row = ResultRow { slot: SLOT, entity_id: ENTITY, place: 1, time_ms: 90_000, best_lap_ms: 30_000, flags: 0 };
    let results = RaceResults { head: head(1, 700, 0), rows: List::new(vec![row]).unwrap() };
    h.reliable(results.clone().into());
    h.frame();
    assert!(h.take_events().contains(&SessionEvent::Results(results.clone())));
    assert_eq!(h.session.last_results(), Some(&results));
}

#[test]
fn players_leaving_and_changing_role_update_the_roster() {
    let mut h = Harness::synced();
    h.reliable(lobby_state(vec![row(0, Role::Racer), row(SLOT, Role::Racer), row(4, Role::Racer)]).into());
    h.reliable(PlayerLeft { head: head(1, 700, 0), slot: 0, reason: LeaveReason::Quit }.into());
    h.reliable(RoleChanged { head: head(2, 700, 0), slot: 4, role: Role::Spectator }.into());
    h.frame();
    assert_eq!(h.session.lobby().racers(), vec![SLOT]);
    assert_eq!(h.session.lobby().players.len(), 2);
}
