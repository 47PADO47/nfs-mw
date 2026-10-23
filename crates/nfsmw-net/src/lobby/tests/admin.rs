use super::*;
use crate::loadout::CarRestriction;
use crate::message::{
    AdminCommand, CancelReason, Kick, KickReason, LeaveReason, Message, Mode, SessionPhase, SettingsPatch, VoteKind,
};
use crate::settings::{Field, SettingsError};

fn kick_of(actions: &[Action]) -> Option<(u8, Kick)> {
    actions.iter().find_map(|action| match action {
        Action::Kick { slot, kick } => Some((*slot, kick.clone())),
        _ => None,
    })
}

#[test]
fn spoofed_host_commands_are_refused_and_flagged() {
    let mut lobby = with_players(2);
    for command in [
        AdminCommand::Kick { slot: 2 },
        AdminCommand::Ban { slot: 0, minutes: 0 },
        AdminCommand::Mute { slot: 2, muted: true },
        AdminCommand::Restart,
        AdminCommand::ChangeMode { mode: Mode::Sprint },
    ] {
        let error = lobby.command(1, command, &Cars, 0, 0).unwrap_err();
        assert_eq!(error, LobbyError::NotPermitted, "{command:?}");
        assert!(error.is_violation());
    }
    assert_eq!(lobby.members().count(), 3);
    assert_eq!(lobby.command(9, AdminCommand::Restart, &Cars, 0, 0), Err(LobbyError::UnknownSlot));
}

#[test]
fn moderation_reaches_only_lower_ranks() {
    let mut lobby = with_players(2);
    lobby.set_admin(0, 1, true).unwrap();
    assert_eq!(lobby.set_admin(1, 2, true), Err(LobbyError::NotPermitted), "only the host names admins");
    assert_eq!(lobby.command(1, AdminCommand::Kick { slot: 0 }, &Cars, 0, 0), Err(LobbyError::BadTarget));
    assert_eq!(lobby.command(1, AdminCommand::Kick { slot: 1 }, &Cars, 0, 0), Err(LobbyError::BadTarget));
    assert_eq!(lobby.command(0, AdminCommand::Kick { slot: 0 }, &Cars, 0, 0), Err(LobbyError::BadTarget));
    assert_eq!(lobby.set_admin(0, 0, false), Err(LobbyError::BadTarget));
    let actions = lobby.command(1, AdminCommand::Kick { slot: 2 }, &Cars, 0, 0).unwrap();
    assert_eq!(kick_of(&actions), Some((2, Kick { reason: KickReason::Kicked, ban_minutes: 0 })));
    assert!(actions.contains(&Action::Left { slot: 2, reason: LeaveReason::Kicked }));
    assert_eq!(lobby.state().players[1].flags & FLAG_ADMIN, FLAG_ADMIN);
}

#[test]
fn a_ban_kicks_and_keeps_the_player_out() {
    let mut lobby = with_players(1);
    let actions = lobby.command(0, AdminCommand::Ban { slot: 1, minutes: 10 }, &Cars, 1_000, 0).unwrap();
    assert_eq!(kick_of(&actions), Some((1, Kick { reason: KickReason::Banned, ban_minutes: 10 })));
    assert!(actions.contains(&Action::Left { slot: 1, reason: LeaveReason::Banned }));
    assert!(lobby.bans().is_banned([1; 16], 600_999));
    assert!(!lobby.bans().is_banned([1; 16], 601_000));
}

#[test]
fn a_server_mute_is_kept_on_the_member() {
    let mut lobby = with_players(1);
    let actions = lobby.command(0, AdminCommand::Mute { slot: 1, muted: true }, &Cars, 0, 0).unwrap();
    assert!(lobby.is_muted(1));
    assert!(broadcasts(&actions).iter().any(|message| matches!(message, Message::SystemNotice(_))));
    lobby.command(0, AdminCommand::Mute { slot: 1, muted: false }, &Cars, 0, 0).unwrap();
    assert!(!lobby.is_muted(1));
}

#[test]
fn a_new_car_restriction_drops_refused_cars_and_cancels_the_start() {
    let mut lobby = with_players(1);
    lobby.select_loadout(1, Loadout::stock(7), &Cars, 0).unwrap();
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    let patch = SettingsPatch { allowed_cars: Some(CarRestriction::Class(1)), ..SettingsPatch::default() };
    let player = lobby.update_settings(1, &patch, &Cars, 0).unwrap_err();
    assert_eq!(player, LobbyError::NotPermitted);
    assert!(player.is_violation());
    let actions = lobby.update_settings(0, &patch, &Cars, 0).unwrap();
    assert!(
        broadcasts(&actions).iter().any(
            |message| matches!(message, Message::CancelCountdown(cancel) if cancel.reason == CancelReason::NotReady)
        )
    );
    assert!(broadcasts(&actions).iter().any(|message| matches!(message, Message::LobbyState(_))));
    let member = lobby.member(1).unwrap();
    assert!(member.loadout.is_none() && !member.ready);
    assert_eq!(lobby.set_ready(1, true, 0, 0), Err(LobbyError::NoLoadout));
    assert_eq!(
        lobby.select_loadout(1, Loadout::stock(8), &Cars, 0),
        Err(LobbyError::Loadout(crate::loadout::LoadoutReject::ClassNotAllowed))
    );
}

#[test]
fn race_settings_are_locked_while_the_race_runs() {
    let mut lobby = with_players(0);
    all_ready(&mut lobby, 0);
    lobby.start(0, 0).unwrap();
    lobby.update(0, 300);
    assert_eq!(lobby.wire_phase(), SessionPhase::Loading);
    let patch = SettingsPatch { traffic_density: Some(10), ..SettingsPatch::default() };
    assert_eq!(
        lobby.update_settings(0, &patch, &Cars, 0),
        Err(LobbyError::Settings(SettingsError::Locked(Field::TrafficDensity)))
    );
    assert_eq!(lobby.select_loadout(0, Loadout::stock(2), &Cars, 0), Err(LobbyError::WrongPhase));
}

#[test]
fn a_mode_change_gives_a_lap_mode_one_lap() {
    let mut lobby = with_players(0);
    lobby.command(0, AdminCommand::ChangeMode { mode: Mode::Circuit }, &Cars, 0, 0).unwrap();
    assert_eq!((lobby.settings().mode, lobby.settings().laps), (Mode::Circuit, 1));
    let refused = lobby.command(0, AdminCommand::ChangeMode { mode: Mode::Drag }, &Cars, 0, 0);
    assert_eq!(refused, Err(LobbyError::Settings(SettingsError::Invalid(Field::Mode))));
}

#[test]
fn passed_votes_are_carried_out() {
    let mut lobby = with_players(2);
    let actions = lobby.apply_vote(VoteKind::Kick, 2, &Cars, 0).unwrap();
    assert_eq!(kick_of(&actions), Some((2, Kick { reason: KickReason::Voted, ban_minutes: 0 })));
    lobby.apply_vote(VoteKind::ChangeTrack, 42, &Cars, 0).unwrap();
    assert_eq!(lobby.settings().track, 42);
    assert_eq!(lobby.apply_vote(VoteKind::SkipResults, 0, &Cars, 0), Err(LobbyError::WrongPhase));
    assert_eq!(lobby.apply_vote(VoteKind::Kick, 300, &Cars, 0), Err(LobbyError::UnknownSlot));
    lobby.apply_vote(VoteKind::ChangeMode, 2, &Cars, 0).unwrap();
    assert_eq!(lobby.settings().mode, Mode::Sprint);
}
