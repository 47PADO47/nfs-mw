use super::*;
use crate::message::{CancelReason, LeaveReason, Message, Role, SessionPhase};
use crate::spectate::SpectateReason;

fn cancelled(actions: &[Action]) -> Option<CancelReason> {
    broadcasts(actions).into_iter().find_map(|message| match message {
        Message::CancelCountdown(cancel) => Some(cancel.reason),
        _ => None,
    })
}

#[test]
fn ready_needs_the_lobby_a_car_and_a_driver() {
    let mut lobby = with_players(1);
    join(&mut lobby, 9, Rank::Player, Placement::Spectator(SpectateReason::Chosen));
    assert_eq!(lobby.set_ready(2, true, 0, 0), Err(LobbyError::Spectator));
    assert_eq!(lobby.set_ready(7, true, 0, 0), Err(LobbyError::UnknownSlot));
    assert!(lobby.set_ready(1, true, 0, 0).is_ok());
    assert!(lobby.set_ready(1, true, 0, 0).unwrap().is_empty(), "no change, no message");
}

#[test]
fn only_the_host_or_an_admin_starts_once_everyone_is_ready() {
    let mut lobby = with_players(2);
    let spoof = lobby.start(1, 0).unwrap_err();
    assert_eq!(spoof, LobbyError::NotPermitted);
    assert!(spoof.is_violation());
    assert_eq!(lobby.start(0, 0), Err(LobbyError::NotAllReady));
    all_ready(&mut lobby, 0);
    let actions = lobby.start(0, 1_000).unwrap();
    assert!(
        matches!(broadcasts(&actions)[0], Message::StartCountdown(start) if start.start_tick == 1_300 && start.seconds == 5)
    );
    assert_eq!(lobby.countdown(), Some(1_300));
    assert_eq!(lobby.start(0, 1_001), Err(LobbyError::CountdownRunning));
}

#[test]
fn the_countdown_is_cancelled_by_unready_leave_host_and_hold() {
    let mut lobby = with_players(2);
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    assert_eq!(cancelled(&lobby.set_ready(1, false, 0, 0).unwrap()), Some(CancelReason::NotReady));
    lobby.set_ready(1, true, 0, 0).unwrap();
    lobby.start(0, 0).unwrap();
    assert_eq!(cancelled(&lobby.remove(2, LeaveReason::Quit).unwrap()), Some(CancelReason::PlayerLeft));
    lobby.start(0, 0).unwrap();
    assert_eq!(lobby.cancel(1), Err(LobbyError::NotPermitted));
    assert_eq!(cancelled(&lobby.cancel(0).unwrap()), Some(CancelReason::HostCancelled));
    assert_eq!(lobby.cancel(0), Err(LobbyError::NoCountdown));
    lobby.start(0, 0).unwrap();
    let held = lobby.hold(0, true).unwrap();
    assert_eq!(cancelled(&held), Some(CancelReason::HostCancelled));
    assert!(held.contains(&Action::Hold { on: true }));
    assert_eq!(lobby.start(0, 0), Err(LobbyError::Held));
    assert_eq!(lobby.state().players[0].flags & FLAG_HOLD, FLAG_HOLD);
    assert!(lobby.hold(0, false).unwrap().contains(&Action::Hold { on: false }));
    assert!(lobby.start(0, 0).is_ok());
}

#[test]
fn a_race_loads_counts_down_to_one_tick_and_runs() {
    let mut lobby = with_players(1);
    join(&mut lobby, 9, Rank::Player, Placement::Spectator(SpectateReason::Chosen));
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    assert!(lobby.update(0, 299).is_empty());
    let loading = lobby.update(0, 300);
    assert!(loading.contains(&Action::Phase { phase: SessionPhase::Loading, target_tick: 0 }));
    let grid: Vec<(u8, u8)> = loading
        .iter()
        .filter_map(|action| match action {
            Action::Send { to, message: Message::LoadWorld(load) } => Some((*to, load.spawn_slot)),
            _ => None,
        })
        .collect();
    assert_eq!(grid, vec![(0, 0), (1, 1), (2, NO_SPAWN_SLOT)]);
    assert!(lobby.member(1).unwrap().loading);
    lobby.load_complete(0).unwrap();
    lobby.load_complete(1).unwrap();
    assert!(lobby.load_complete(2).unwrap().is_empty(), "a spectator is not in the barrier");
    lobby.server_loaded(5_000).unwrap();
    let countdown = lobby.update(5_000, 400);
    assert!(countdown.contains(&Action::Phase { phase: SessionPhase::Countdown, target_tick: 580 }));
    assert!(countdown.contains(&Action::Broadcast(Message::StartAt(crate::message::StartAt { start_tick: 580 }))));
    assert!(lobby.update(5_100, 579).is_empty());
    let running = lobby.update(5_200, 580);
    assert!(running.contains(&Action::Phase { phase: SessionPhase::Running, target_tick: 0 }));
}

#[test]
fn a_racer_still_loading_at_the_deadline_watches_this_race() {
    let mut lobby = with_players(1);
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    lobby.update(0, 300);
    lobby.load_complete(0).unwrap();
    lobby.server_loaded(1_000).unwrap();
    assert!(lobby.update(60_000, 301).is_empty(), "the barrier waits");
    let actions = lobby.update(121_000, 302);
    let to_late = actions.iter().find_map(|action| match action {
        Action::Send { to: 1, message: Message::CancelCountdown(cancel) } => Some(cancel.reason),
        _ => None,
    });
    assert_eq!(to_late, Some(CancelReason::LoadTimeout));
    let member = lobby.member(1).unwrap();
    assert_eq!((member.role, member.wants), (Role::Spectator, Role::Racer));
}

#[test]
fn results_return_to_the_lobby_and_seat_the_waiting() {
    let mut lobby = with_players(1);
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    lobby.update(0, 300);
    lobby.server_loaded(0).unwrap();
    lobby.load_complete(0).unwrap();
    lobby.load_complete(1).unwrap();
    lobby.update(0, 301);
    lobby.update(0, 481);
    join(&mut lobby, 7, Rank::Player, Placement::Spectator(SpectateReason::WaitingForRace));
    assert!(lobby.finish().unwrap().contains(&Action::Phase { phase: SessionPhase::Results, target_tick: 0 }));
    let actions = lobby.back_to_lobby().unwrap();
    assert!(actions.contains(&Action::RoleChanged { slot: 2, role: Role::Racer }));
    assert!(lobby.members().all(|member| !member.ready));
    assert_eq!(lobby.racers(), 3);
}

#[test]
fn auto_start_begins_when_the_last_racer_is_ready() {
    let config = LobbyConfig { auto_start: true, ..LobbyConfig::default() };
    let mut lobby = lobby_with(Settings::lan(ServerName::new("auto").unwrap()), config);
    join(&mut lobby, 0, Rank::Host, Placement::Racer);
    join(&mut lobby, 1, Rank::Player, Placement::Racer);
    assert!(lobby.set_ready(0, true, 0, 10).unwrap().len() == 1);
    let actions = lobby.set_ready(1, true, 0, 10).unwrap();
    assert!(broadcasts(&actions).iter().any(|message| matches!(message, Message::StartCountdown(_))));
    assert_eq!(lobby.countdown(), Some(310));
}

#[test]
fn lobby_actions_are_rate_limited() {
    let mut lobby = with_players(1);
    let results: Vec<bool> = (0..8).map(|index| lobby.set_ready(1, index % 2 == 0, 0, 0).is_ok()).collect();
    assert_eq!(results, vec![true, true, true, true, true, true, false, false]);
    assert!(lobby.set_ready(1, true, 500, 0).is_ok());
}

#[test]
fn a_restart_goes_back_to_the_lobby_and_counts_down_again() {
    let mut lobby = with_players(1);
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    lobby.update(0, 300);
    lobby.server_loaded(0).unwrap();
    lobby.load_complete(0).unwrap();
    lobby.load_complete(1).unwrap();
    lobby.update(0, 301);
    lobby.update(0, 481);
    let actions = lobby.command(0, crate::message::AdminCommand::Restart, &Cars, 0, 2_000).unwrap();
    assert!(actions.contains(&Action::Phase { phase: SessionPhase::Lobby, target_tick: 0 }));
    assert_eq!(lobby.countdown(), Some(2_300));
    assert!(lobby.members().all(|member| member.ready));
}

#[test]
fn a_spectator_may_ask_to_race_next_and_switch_in_the_lobby() {
    let mut lobby = with_players(0);
    let slot = join(&mut lobby, 4, Rank::Player, Placement::Spectator(SpectateReason::Chosen));
    assert_eq!(lobby.set_role(slot, Role::Cop, 0), Err(LobbyError::RoleNotOffered));
    let actions = lobby.set_role(slot, Role::Racer, 0).unwrap();
    assert!(actions.contains(&Action::RoleChanged { slot, role: Role::Racer }));
    assert!(lobby.set_role(slot, Role::Spectator, 0).is_ok());
    let mut watcher = member(5, Rank::Player, Placement::Spectator(SpectateReason::Chosen));
    watcher.spectate_only = true;
    let (only, _) = lobby.add(watcher, 0).unwrap();
    assert_eq!(lobby.set_role(only, Role::Racer, 0), Err(LobbyError::SpectateOnly));
}
