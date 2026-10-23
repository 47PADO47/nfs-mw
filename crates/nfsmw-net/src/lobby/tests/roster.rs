use super::*;
use crate::message::{LeaveReason, LobbyChange, Message, Role, SessionPhase};
use crate::spectate::SpectateReason;

#[test]
fn joiners_take_the_lowest_free_slot_and_get_the_lobby() {
    let mut lobby = lobby();
    let (slot, actions) = lobby.add(member(0, Rank::Host, Placement::Racer), 0).unwrap();
    assert_eq!(slot, 0);
    assert!(
        matches!(&broadcasts(&actions)[0], Message::LobbyDelta(delta) if matches!(delta.changes[0], LobbyChange::Add(_)))
    );
    assert!(actions.contains(&Action::Joined { slot: 0 }));
    assert!(actions.iter().any(|action| matches!(action, Action::Send { to: 0, message: Message::LobbyState(_) })));
    join(&mut lobby, 1, Rank::Player, Placement::Racer);
    join(&mut lobby, 2, Rank::Player, Placement::Racer);
    lobby.remove(1, LeaveReason::Quit).unwrap();
    assert_eq!(join(&mut lobby, 3, Rank::Player, Placement::Racer), 1);
    let state = lobby.state();
    assert_eq!(state.players.len(), 3);
    assert_eq!(state.host_slot, 0);
    assert_eq!(state.players[0].flags & FLAG_HOST, FLAG_HOST);
}

#[test]
fn a_full_lobby_refuses_racers_but_counts_spectators_apart() {
    let mut settings = Settings::lan(ServerName::new("small").unwrap());
    settings.max_players = 2;
    settings.max_spectators = 1;
    let mut lobby = lobby_with(settings, LobbyConfig::default());
    join(&mut lobby, 0, Rank::Host, Placement::Racer);
    join(&mut lobby, 1, Rank::Player, Placement::Racer);
    assert_eq!(lobby.add(member(2, Rank::Player, Placement::Racer), 0).unwrap_err(), LobbyError::Full);
    let watcher = Placement::Spectator(SpectateReason::Chosen);
    join(&mut lobby, 3, Rank::Player, watcher);
    assert_eq!(lobby.add(member(4, Rank::Player, watcher), 0).unwrap_err(), LobbyError::Full);
    assert_eq!((lobby.racers(), lobby.spectators()), (2, 1));
}

#[test]
fn sixteen_racers_and_eight_spectators_fit_the_id_space() {
    let mut settings = Settings::lan(ServerName::new("big").unwrap());
    settings.max_players = 16;
    settings.max_spectators = 8;
    let mut lobby = lobby_with(settings, LobbyConfig::default());
    for id in 0..16 {
        join(&mut lobby, id, Rank::Player, Placement::Racer);
    }
    for id in 16..24 {
        join(&mut lobby, id, Rank::Player, Placement::Spectator(SpectateReason::Chosen));
    }
    assert_eq!((lobby.racers(), lobby.spectators()), (16, 8));
    assert_eq!(lobby.add(member(30, Rank::Player, Placement::Racer), 0).unwrap_err(), LobbyError::Full);
}

#[test]
fn the_host_leaving_ends_a_listen_session() {
    let mut lobby = with_players(1);
    let player = lobby.remove(1, LeaveReason::Timeout).unwrap();
    assert!(player.contains(&Action::Left { slot: 1, reason: LeaveReason::Timeout }));
    assert!(!player.contains(&Action::EndSession));
    let host = lobby.remove(0, LeaveReason::Quit).unwrap();
    assert_eq!(host.last(), Some(&Action::EndSession));
    assert_eq!(lobby.remove(0, LeaveReason::Quit), Err(LobbyError::UnknownSlot));
}

#[test]
fn a_racer_joining_a_running_free_roam_spawns() {
    let mut lobby = with_players(0);
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    lobby.update(0, 300);
    lobby.server_loaded(0).unwrap();
    lobby.load_complete(0).unwrap();
    lobby.update(10, 301);
    lobby.update(20, 481);
    assert_eq!(lobby.wire_phase(), SessionPhase::Running);
    let (slot, actions) = lobby.add(member(5, Rank::Player, Placement::Racer), 30).unwrap();
    assert!(actions.contains(&Action::Spawn { slot }));
}

#[test]
fn an_eliminated_racer_watches_outside_the_limit_and_races_next_time() {
    let mut settings = Settings::lan(ServerName::new("ko").unwrap());
    settings.max_spectators = 0;
    let mut lobby = lobby_with(settings, LobbyConfig::default());
    join(&mut lobby, 0, Rank::Host, Placement::Racer);
    join(&mut lobby, 1, Rank::Player, Placement::Racer);
    assert_eq!(lobby.make_spectator(1, SpectateReason::Chosen), Err(LobbyError::Full));
    let actions = lobby.make_spectator(1, SpectateReason::Eliminated).unwrap();
    assert!(actions.contains(&Action::RoleChanged { slot: 1, role: Role::Spectator }));
    assert!(actions.contains(&Action::Despawn { slot: 1 }));
    let member = lobby.member(1).unwrap();
    assert_eq!((member.role, member.wants), (Role::Spectator, Role::Racer));
    assert_eq!(member.row(false).flags & FLAG_WAITING, FLAG_WAITING);
    assert_eq!(lobby.voters(), vec![0], "a spectator does not vote");
}

#[test]
fn a_dropped_member_is_flagged_away_and_unready() {
    let mut lobby = with_players(1);
    lobby.set_ready(1, true, 0, 0).unwrap();
    let actions = lobby.set_away(1, true).unwrap();
    assert!(!lobby.member(1).unwrap().ready);
    assert!(matches!(&broadcasts(&actions)[0], Message::LobbyDelta(delta)
        if matches!(&delta.changes[0], LobbyChange::Update(row) if row.flags & FLAG_AWAY != 0)));
    assert_eq!(lobby.voters(), vec![0]);
    assert_eq!(lobby.racers(), 2, "a held slot still counts");
}

#[test]
fn session_data_carries_every_car() {
    let lobby = with_players(2);
    let data = lobby.session_data();
    assert_eq!(data.lobby.players.len(), 3);
    assert_eq!(data.loadouts.len(), 3);
    assert_eq!(crate::drop::SessionData::decode(&data.encode()), Ok(data));
}
