use super::*;

const MEMBERS: [u8; 6] = [0, 1, 2, 3, 4, 5];

fn roll() -> VoteRoll<'static> {
    VoteRoll { electorate: &MEMBERS, members: &MEMBERS, protected: &[0] }
}

fn vote(kind: VoteKind, option: u32) -> Vote {
    Vote { kind, option }
}

fn passed(events: &[VoteEvent]) -> Option<(VoteKind, u32)> {
    events.iter().find_map(|event| match event {
        VoteEvent::Passed { kind, option } => Some((*kind, *option)),
        _ => None,
    })
}

#[test]
fn a_restart_passes_on_a_strict_majority() {
    let mut box_ = VoteBox::new();
    let restart = vote(VoteKind::Restart, 0);
    let events = box_.submit(1, &restart, &roll(), 100, 0).unwrap();
    assert!(
        matches!(&events[0], VoteEvent::State(state) if state.options[0].count == 1 && state.ends_tick == 100 + VOTE_TICKS)
    );
    assert!(passed(&box_.submit(2, &restart, &roll(), 101, 0).unwrap()).is_none());
    assert!(passed(&box_.submit(3, &restart, &roll(), 102, 0).unwrap()).is_none(), "3 of 6 is not a majority");
    assert_eq!(passed(&box_.submit(4, &restart, &roll(), 103, 0).unwrap()), Some((VoteKind::Restart, 0)));
    assert!(box_.state().is_none());
}

#[test]
fn a_vote_times_out_as_failed() {
    let mut box_ = VoteBox::new();
    box_.submit(1, &vote(VoteKind::SkipResults, 0), &roll(), 0, 0).unwrap();
    assert!(box_.update(VOTE_TICKS - 1).is_empty());
    assert_eq!(box_.update(VOTE_TICKS), vec![VoteEvent::Failed { kind: VoteKind::SkipResults, option: 0 }]);
    assert!(box_.update(VOTE_TICKS + 1).is_empty());
}

#[test]
fn stuffing_is_refused() {
    let mut box_ = VoteBox::new();
    let restart = vote(VoteKind::Restart, 0);
    box_.submit(1, &restart, &roll(), 0, 0).unwrap();
    assert_eq!(box_.submit(1, &restart, &roll(), 0, 0), Err(VoteError::AlreadyVoted));
    assert_eq!(box_.submit(9, &restart, &roll(), 0, 0), Err(VoteError::NotEligible), "a spoofed slot");
    let late = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    let joined_later = VoteRoll { electorate: &late, ..roll() };
    assert_eq!(box_.submit(7, &restart, &joined_later, 0, 0), Err(VoteError::NotEligible), "the electorate is fixed");
    assert_eq!(box_.submit(2, &vote(VoteKind::Kick, 3), &roll(), 0, 0), Err(VoteError::OtherKind));
    assert_eq!(box_.submit(2, &vote(VoteKind::Restart, 7), &roll(), 0, 0), Err(VoteError::BadOption));
    assert_eq!(box_.state().unwrap().options[0].count, 1);
}

#[test]
fn a_kick_leaves_out_its_target_and_spares_the_host() {
    let mut box_ = VoteBox::new();
    assert_eq!(box_.submit(1, &vote(VoteKind::Kick, 0), &roll(), 0, 0), Err(VoteError::BadTarget), "the host");
    assert_eq!(box_.submit(1, &vote(VoteKind::Kick, 1), &roll(), 0, 0), Err(VoteError::BadTarget), "oneself");
    assert_eq!(box_.submit(1, &vote(VoteKind::Kick, 20), &roll(), 0, 0), Err(VoteError::BadTarget), "nobody");
    assert_eq!(box_.submit(1, &vote(VoteKind::Kick, 300), &roll(), 0, 0), Err(VoteError::BadTarget));
    box_.submit(1, &vote(VoteKind::Kick, 5), &roll(), 0, 0).unwrap();
    assert_eq!(box_.submit(5, &vote(VoteKind::Kick, 5), &roll(), 0, 0), Err(VoteError::NotEligible));
    box_.submit(2, &vote(VoteKind::Kick, 5), &roll(), 0, 0).unwrap();
    let events = box_.submit(3, &vote(VoteKind::Kick, 5), &roll(), 0, 0).unwrap();
    assert_eq!(passed(&events), Some((VoteKind::Kick, 5)), "3 of the 5 voters left");
}

#[test]
fn a_kick_target_leaving_cancels_the_vote() {
    let mut box_ = VoteBox::new();
    box_.submit(1, &vote(VoteKind::Kick, 4), &roll(), 0, 0).unwrap();
    assert_eq!(box_.leave(4), vec![VoteEvent::Cancelled { kind: VoteKind::Kick, option: 4 }]);
    assert!(box_.state().is_none());
}

#[test]
fn a_voter_leaving_can_decide_the_vote() {
    let mut box_ = VoteBox::new();
    let restart = vote(VoteKind::Restart, 0);
    box_.submit(0, &restart, &roll(), 0, 0).unwrap();
    box_.submit(1, &restart, &roll(), 0, 0).unwrap();
    box_.submit(2, &restart, &roll(), 0, 0).unwrap();
    assert!(passed(&box_.leave(5)).is_some(), "3 of 5");
}

#[test]
fn a_vote_fails_early_when_no_majority_is_possible() {
    let mut box_ = VoteBox::new();
    let five = [1, 2, 3, 4, 5];
    let roll = VoteRoll { electorate: &five, members: &five, protected: &[] };
    box_.submit(1, &vote(VoteKind::ChangeTrack, 10), &roll, 0, 0).unwrap();
    box_.submit(2, &vote(VoteKind::ChangeTrack, 11), &roll, 0, 0).unwrap();
    box_.submit(3, &vote(VoteKind::ChangeTrack, 12), &roll, 0, 0).unwrap();
    let events = box_.submit(4, &vote(VoteKind::ChangeTrack, 13), &roll, 0, 0).unwrap();
    assert!(events.contains(&VoteEvent::Failed { kind: VoteKind::ChangeTrack, option: 10 }));
}

#[test]
fn track_and_mode_votes_gather_bounded_options() {
    let mut box_ = VoteBox::new();
    let many: Vec<u8> = (0..16).collect();
    let roll = VoteRoll { electorate: &many, members: &many, protected: &[] };
    for slot in 0..8u8 {
        box_.submit(slot, &vote(VoteKind::ChangeTrack, u32::from(slot) + 100), &roll, 0, 0).unwrap();
    }
    assert_eq!(box_.submit(8, &vote(VoteKind::ChangeTrack, 999), &roll, 0, 0), Err(VoteError::TooManyOptions));
    let mut modes = VoteBox::new();
    assert_eq!(modes.submit(0, &vote(VoteKind::ChangeMode, 50), &roll, 0, 0), Err(VoteError::BadOption));
    modes.submit(0, &vote(VoteKind::ChangeMode, 1), &roll, 0, 0).unwrap();
    assert_eq!(modes.submit(1, &vote(VoteKind::ChangeMode, 9), &roll, 0, 0), Err(VoteError::BadOption));
}

#[test]
fn a_player_starts_one_vote_a_minute() {
    let mut box_ = VoteBox::new();
    let skip = vote(VoteKind::SkipResults, 0);
    box_.submit(1, &skip, &roll(), 0, 0).unwrap();
    box_.cancel();
    assert_eq!(box_.submit(1, &skip, &roll(), 0, 59_999), Err(VoteError::RateLimited));
    assert!(box_.submit(2, &skip, &roll(), 0, 59_999).is_ok());
    box_.cancel();
    assert!(box_.submit(1, &skip, &roll(), 0, 60_000).is_ok());
}

#[test]
fn spectators_outside_the_electorate_cannot_start() {
    let mut box_ = VoteBox::new();
    let voters = [1, 2];
    let members = [1, 2, 3];
    let roll = VoteRoll { electorate: &voters, members: &members, protected: &[] };
    assert_eq!(box_.submit(3, &vote(VoteKind::Restart, 0), &roll, 0, 0), Err(VoteError::NotEligible));
    assert!(box_.submit(1, &vote(VoteKind::Kick, 3), &roll, 0, 0).is_ok(), "a spectator can be kicked");
}
