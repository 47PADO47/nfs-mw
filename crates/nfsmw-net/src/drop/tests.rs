use super::*;
use crate::loadout::Loadout;
use crate::message::{
    BulkChunk, BulkKind, Bytes, LeaveReason, List, LobbyState, MAX_BULK_CHUNK, Message, ServerName, SessionPhase,
    Settings,
};

const ME: [u8; 16] = [7; 16];

fn context(resumable: bool) -> DropContext {
    DropContext { has_car: true, racing: true, in_pursuit: true, resumable }
}

#[test]
fn presence_marks_lag_then_timeout() {
    let mut presence = Presence::new();
    presence.heard(3, 1_000);
    presence.heard(3, 500);
    assert_eq!(presence.liveness(3, 2_999), Some(Liveness::Fresh));
    assert_eq!(presence.liveness(3, 3_000), Some(Liveness::Lagging));
    assert_eq!(presence.liveness(3, 11_000), Some(Liveness::TimedOut));
    assert_eq!(presence.liveness(4, 0), None);
    presence.heard(4, 5_000);
    assert_eq!(presence.take_timed_out(11_000), vec![3]);
    assert_eq!(presence.liveness(3, 11_000), None);
    presence.forget(4);
    assert!(presence.take_timed_out(u64::MAX).is_empty());
}

#[test]
fn a_timeout_holds_the_slot_and_ghosts_the_car() {
    let actions = on_drop(2, DropCause::Timeout, &context(true), 1_000);
    assert_eq!(actions[0], DropAction::LeavePursuit { slot: 2 });
    assert!(matches!(actions[1], DropAction::Notice(_)));
    assert_eq!(actions[2], DropAction::GhostCar { slot: 2 });
    assert_eq!(actions[3], DropAction::HoldSlot { slot: 2, until_ms: 31_000 });
    assert_eq!(actions.len(), 4, "no result and no release yet");
    let expiry = on_expiry(2, &context(true));
    assert_eq!(
        expiry,
        vec![
            DropAction::RemoveCar { slot: 2 },
            DropAction::DidNotFinish { slot: 2 },
            DropAction::ReleaseSlot { slot: 2, reason: LeaveReason::Timeout },
        ]
    );
}

#[test]
fn a_quit_or_kick_releases_at_once() {
    let quit = on_drop(1, DropCause::Quit, &context(true), 0);
    assert!(quit.contains(&DropAction::RemoveCar { slot: 1 }));
    assert!(quit.contains(&DropAction::ReleaseSlot { slot: 1, reason: LeaveReason::Quit }));
    assert!(!quit.iter().any(|action| matches!(action, DropAction::HoldSlot { .. })));
    let kick = on_drop(1, DropCause::Banned, &context(true), 0);
    assert!(!kick.iter().any(|action| matches!(action, DropAction::Notice(_))), "the lobby sends the kick notice");
    assert_eq!(kick.last(), Some(&DropAction::ReleaseSlot { slot: 1, reason: LeaveReason::Banned }));
    let lobby = on_drop(
        1,
        DropCause::Timeout,
        &DropContext { has_car: false, racing: false, in_pursuit: false, resumable: false },
        0,
    );
    assert_eq!(lobby.len(), 2, "a notice and the release");
}

#[test]
fn a_resume_restores_the_car_with_a_full_snapshot() {
    let actions = on_resume(5, true);
    assert_eq!(actions[0], DropAction::RestoreCar { slot: 5 });
    assert_eq!(actions[1], DropAction::FullSnapshot { slot: 5 });
    assert_eq!(on_resume(5, false).len(), 2);
}

#[test]
fn a_token_resumes_once_inside_the_window() {
    let mut book = ResumeBook::new();
    let token = book.issue(4, ME, [9; 32]);
    assert_eq!(book.hold(4, 1_000), Some(31_000));
    assert!(book.is_held(4));
    assert_eq!(book.resume(&token, ME, 30_999), Ok(Resumed { slot: 4, was_live: false }));
    assert_eq!(book.resume(&token, ME, 31_000), Err(ResumeError::Unknown), "replay");
    assert!(!book.is_held(4));
}

#[test]
fn a_token_after_the_window_is_expired_and_the_slot_is_returned() {
    let mut book = ResumeBook::new();
    let token = book.issue(4, ME, [9; 32]);
    book.hold(4, 0);
    assert_eq!(book.resume(&token, ME, RESUME_WINDOW_MS), Err(ResumeError::Expired));
    assert_eq!(book.take_expired(RESUME_WINDOW_MS - 1), Vec::<u8>::new());
    assert_eq!(book.take_expired(RESUME_WINDOW_MS), vec![4]);
    assert_eq!(book.resume(&token, ME, 0), Err(ResumeError::Unknown));
}

#[test]
fn a_stolen_or_forged_token_is_refused_without_burning_it() {
    let mut book = ResumeBook::new();
    let token = book.issue(4, ME, [9; 32]);
    book.hold(4, 0);
    assert_eq!(book.resume(&token, [8; 16], 10), Err(ResumeError::Unknown), "another player");
    assert_eq!(book.resume(&[9; 31], ME, 10), Err(ResumeError::Unknown), "wrong length");
    assert_eq!(book.resume(&[1; 32], ME, 10), Err(ResumeError::Unknown), "never issued");
    assert!(book.resume(&token, ME, 10).is_ok(), "the owner still resumes");
}

#[test]
fn a_live_slot_can_be_taken_over_and_a_reissue_replaces_the_token() {
    let mut book = ResumeBook::new();
    let old = book.issue(1, ME, [1; 32]);
    let new = book.issue(1, ME, [2; 32]);
    assert_eq!(book.resume(&old, ME, 0), Err(ResumeError::Unknown));
    assert_eq!(book.resume(&new, ME, 0), Ok(Resumed { slot: 1, was_live: true }));
    assert_eq!(book.hold(1, 0), None, "a used token holds nothing");
    book.issue(2, ME, [3; 32]);
    book.release(2);
    assert_eq!(book.hold(2, 0), None);
}

fn blob(len: usize) -> Vec<u8> {
    (0..len).map(|index| (index * 31 % 251) as u8).collect()
}

#[test]
fn a_transfer_splits_into_ordered_chunks_and_reassembles() {
    let data = blob(3 * MAX_BULK_CHUNK + 100);
    let mut sender = BulkSender::new(BulkKind::SessionData, 7, data.clone()).unwrap();
    let mut receiver = BulkReceiver::new(SESSION_DATA_LIMIT);
    let mut done = None;
    let mut calls = 0;
    while !sender.is_done() {
        calls += 1;
        for message in sender.next(1_500) {
            if let Some(complete) = receiver.accept(&message).unwrap() {
                done = Some(complete);
            }
        }
    }
    assert_eq!(calls, 3, "1 500 bytes a call");
    assert_eq!(done, Some(Complete { kind: BulkKind::SessionData, id: 7, data }));
    assert!(sender.next(1_000).is_empty());
    assert!(!receiver.is_open());
}

#[test]
fn an_empty_transfer_is_a_start_and_an_end() {
    let mut sender = BulkSender::new(BulkKind::ModManifest, 1, Vec::new()).unwrap();
    let messages = sender.next(0);
    assert_eq!(messages.len(), 2);
    let mut receiver = BulkReceiver::new(10);
    assert!(receiver.accept(&messages[0]).unwrap().is_none());
    assert_eq!(receiver.accept(&messages[1]).unwrap().unwrap().data, Vec::<u8>::new());
}

#[test]
fn a_receiver_refuses_hostile_transfers() {
    let data = blob(2_000);
    let messages = BulkSender::new(BulkKind::SessionData, 3, data).unwrap().next(usize::MAX);
    let mut small = BulkReceiver::new(1_000);
    assert_eq!(small.accept(&messages[0]), Err(BulkError::TooLarge));
    let mut receiver = BulkReceiver::new(SESSION_DATA_LIMIT);
    assert_eq!(receiver.accept(&messages[1]), Err(BulkError::NotStarted));
    receiver.accept(&messages[0]).unwrap();
    assert_eq!(receiver.accept(&messages[0]), Err(BulkError::Busy));
    receiver.accept(&messages[0]).unwrap();
    assert_eq!(receiver.accept(&messages[2]), Err(BulkError::OutOfOrder), "chunk 2 before chunk 1");
    receiver.accept(&messages[0]).unwrap();
    receiver.accept(&messages[1]).unwrap();
    let end = messages.last().unwrap();
    assert_eq!(receiver.accept(end), Err(BulkError::Short));
    let mut wrong = receiver.clone();
    wrong.accept(&messages[0]).unwrap();
    let other = Message::BulkChunk(BulkChunk { id: 4, offset: 0, bytes: Bytes::new(vec![0; 4]).unwrap() });
    assert_eq!(wrong.accept(&other), Err(BulkError::WrongId));
}

#[test]
fn a_tampered_or_overlong_transfer_is_refused() {
    let messages = BulkSender::new(BulkKind::SessionData, 3, blob(100)).unwrap().next(usize::MAX);
    let mut receiver = BulkReceiver::new(SESSION_DATA_LIMIT);
    receiver.accept(&messages[0]).unwrap();
    let tampered = Message::BulkChunk(BulkChunk { id: 3, offset: 0, bytes: Bytes::new(vec![0; 100]).unwrap() });
    receiver.accept(&tampered).unwrap();
    assert_eq!(receiver.accept(&messages[2]), Err(BulkError::HashMismatch));
    receiver.accept(&messages[0]).unwrap();
    let long = Message::BulkChunk(BulkChunk { id: 3, offset: 0, bytes: Bytes::new(vec![0; 101]).unwrap() });
    assert_eq!(receiver.accept(&long), Err(BulkError::Overrun));
}

#[test]
fn session_data_round_trips_through_a_transfer() {
    let lobby = LobbyState {
        players: List::empty(),
        settings: Settings::lan(ServerName::new("Rockport").unwrap()),
        phase: SessionPhase::Running,
        host_slot: 0,
    };
    let row = LoadoutRow { slot: 2, loadout: Loadout::stock(0x1234) };
    let data = SessionData { lobby, loadouts: List::new(vec![row]).unwrap() };
    let bytes = data.encode();
    assert_eq!(SessionData::decode(&bytes), Ok(data));
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(SessionData::decode(&longer).is_err());
}
