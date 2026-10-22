use blackbox_net_core::endpoint::reason;

use super::*;
use crate::client_session::{EndReason, Status};
use crate::drop::{BulkSender, LoadoutRow, SessionData};
use crate::message::{
    BulkKind, CancelCountdown, CancelReason, ChatChannel, ChatMessage, ChatText, List, LoadWorld, LobbyChange,
    LobbyDelta, LobbyPlayer, LobbyState, Mode, NoticeCode, ResetRequest, Role, SessionPhase, SetReady, StartAt,
    StartCountdown, SystemNotice, Vote, VoteKind, VoteOption, VoteState,
};

pub fn row(slot: u8, role: Role) -> LobbyPlayer {
    LobbyPlayer {
        slot,
        player_id: [slot; 16],
        name: Name::new(&format!("p{slot}")).unwrap(),
        car_type: 3,
        ready: false,
        role,
        team: 0,
        ping_ms: 10,
        flags: 0,
    }
}

pub fn lobby_state(rows: Vec<LobbyPlayer>) -> LobbyState {
    LobbyState {
        players: List::new(rows).unwrap(),
        settings: Settings::lan(ServerName::new("room").unwrap()),
        phase: SessionPhase::Lobby,
        host_slot: 0,
    }
}

#[test]
fn the_roster_follows_state_and_deltas() {
    let mut h = Harness::joined();
    h.reliable(lobby_state(vec![row(0, Role::Racer), row(SLOT, Role::Racer)]).into());
    h.frame();
    assert_eq!(h.take_events(), vec![SessionEvent::LobbyChanged]);
    assert_eq!(h.session.lobby().players.len(), 2);
    assert!(!h.session.lobby().is_host());

    let mut ready = row(SLOT, Role::Racer);
    ready.ready = true;
    let changes =
        vec![LobbyChange::Add(row(5, Role::Cop)), LobbyChange::Update(ready), LobbyChange::Remove { slot: 0 }];
    h.reliable(LobbyDelta { changes: List::new(changes).unwrap() }.into());
    h.frame();
    let lobby = h.session.lobby();
    assert_eq!(lobby.players.iter().map(|row| row.slot).collect::<Vec<_>>(), vec![SLOT, 5]);
    assert!(lobby.me().unwrap().ready);
    assert_eq!(lobby.racers(), vec![SLOT]);
}

#[test]
fn countdown_load_start_vote_chat_and_notices_are_mirrored() {
    let mut h = Harness::joined();
    let countdown = StartCountdown { start_tick: 900, seconds: 5 };
    let load = LoadWorld {
        track: 4,
        mode: Mode::Circuit,
        laps: 3,
        time_of_day: 600,
        weather: 0,
        traffic_density: 20,
        cop_level_cap: 2,
        pursuits: false,
        spawn_slot: 1,
    };
    let vote = VoteState {
        kind: VoteKind::Kick,
        options: List::new(vec![VoteOption { id: 4, count: 1 }]).unwrap(),
        ends_tick: 2000,
    };
    let chat = ChatMessage { from_slot: 0, channel: ChatChannel::All, text: ChatText::new("hi").unwrap(), tick: 5 };
    let notice = SystemNotice { code: NoticeCode::Join, args: List::new(vec![3]).unwrap() };
    h.reliable(countdown.clone().into());
    h.reliable(CancelCountdown { reason: CancelReason::NotReady }.into());
    h.reliable(countdown.clone().into());
    h.reliable(load.clone().into());
    h.reliable(StartAt { start_tick: 1200 }.into());
    h.reliable(vote.clone().into());
    h.reliable(chat.clone().into());
    h.reliable(notice.clone().into());
    h.frame();
    let events = h.take_events();
    assert_eq!(events.len(), 8);
    assert_eq!(events[1], SessionEvent::CountdownCancelled(CancelReason::NotReady));
    assert_eq!(events[4], SessionEvent::StartAt(1200));
    let lobby = h.session.lobby();
    assert_eq!(lobby.countdown, Some(countdown));
    assert_eq!(lobby.load, Some(load));
    assert_eq!(lobby.start_tick, Some(1200));
    assert_eq!(lobby.vote, Some(vote));
    assert_eq!(lobby.chat.back().map(|line| line.text.as_str()), Some("hi"));
    assert_eq!(lobby.notices.back(), Some(&notice));
}

#[test]
fn chat_keeps_the_newest_lines() {
    let mut config = config();
    config.chat_lines = 3;
    let mut h = Harness::new(config);
    h.connect();
    h.reliable(accepted(SLOT, b"t").into());
    for index in 0..5u32 {
        let text = ChatText::new(&format!("line {index}")).unwrap();
        h.reliable(ChatMessage { from_slot: 1, channel: ChatChannel::All, text, tick: index }.into());
    }
    h.frame();
    let lines: Vec<_> = h.session.lobby().chat.iter().map(|line| line.tick).collect();
    assert_eq!(lines, vec![2, 3, 4]);
}

#[test]
fn late_join_session_data_arrives_in_chunks() {
    let mut h = Harness::joined();
    let rows = (0..8).map(|slot| row(slot, Role::Racer)).collect();
    let loadouts = (0..8).map(|slot| LoadoutRow { slot, loadout: Loadout::stock(u32::from(slot)) }).collect();
    let data = SessionData { lobby: lobby_state(rows), loadouts: List::new(loadouts).unwrap() };
    let mut sender = BulkSender::new(BulkKind::SessionData, 1, data.encode()).unwrap();
    while !sender.is_done() {
        for message in sender.next(100) {
            h.reliable(message);
        }
    }
    h.frame();
    assert!(h.take_events().contains(&SessionEvent::SessionData));
    assert_eq!(h.session.lobby().players.len(), 8);
    assert_eq!(h.session.lobby().loadout(5).map(|row| row.loadout.car_type), Some(5));
}

#[test]
fn commands_send_their_messages() {
    let mut h = Harness::synced();
    h.sent();
    assert!(h.session.set_ready(true));
    assert!(h.session.vote(VoteKind::Restart, 0));
    assert!(h.session.say("hello").unwrap());
    assert!(h.session.say("").is_err());
    assert!(h.session.request_reset());
    assert!(h.session.choose_loadout(Loadout::stock(9)));
    let sent = h.sent_messages();
    assert_eq!(sent[0], SetReady { ready: true }.into());
    assert_eq!(sent[1], Vote { kind: VoteKind::Restart, option: 0 }.into());
    assert!(matches!(&sent[2], Message::ChatSend(chat) if chat.text.as_str() == "hello"));
    assert!(matches!(&sent[3], Message::ResetRequest(ResetRequest { tick }) if *tick > 600));
    assert!(matches!(&sent[4], Message::SelectLoadout(select) if select.loadout.car_type == 9));

    h.session.leave();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::Left));
    assert_eq!(h.peer.closed(), Some(reason::NORMAL));
    assert!(!h.session.set_ready(false), "nothing goes out after leaving");
}
