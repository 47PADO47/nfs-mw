//! Votes for a kick, a restart, a mode, a track or skipping the results (MP-27). The server counts.
//!
//! One vote runs at a time. The first `Vote` message starts it with the sender's option and counts the sender's vote;
//! later `Vote` messages of the same kind are votes. Rules:
//!
//! - **Electorate.** Fixed when the vote starts (the caller passes the eligible slots, normally every racer and cop),
//!   so a player who joins during the vote cannot vote; a kick's target is left out. A voter who leaves is removed.
//! - **One vote per player.** A second vote from the same slot is refused, even for another option.
//! - **Simple majority of the electorate.** An option passes once more than half of the electorate voted for it; not
//!   voting counts against. The vote fails at its end tick ([`VOTE_TICKS`], 30 s) or as soon as no option can still
//!   reach a majority.
//! - **Options.** A kick names a member slot that is not the starter and not protected (the host); restart and skip
//!   results take option 0 only; a mode vote takes a known mode code; a track vote any id. Mode and track votes may
//!   gather up to [`MAX_VOTE_OPTIONS`] options; the other kinds have only the starter's.
//! - **Rate.** One vote started per player per 60 s ([`crate::lobby::VOTE_START_RATE`]); casting is a lobby action
//!   and rate-limited by the lobby.

mod ballot;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use ballot::Ballot;

use crate::lobby::{RateLimit, VOTE_START_RATE};
use crate::message::{MAX_SLOTS, MAX_VOTE_OPTIONS, Mode, Vote, VoteKind, VoteState};

/// How long a vote runs, in ticks: 30 s.
pub const VOTE_TICKS: u32 = 30 * 60;

/// Who may take part in a vote that starts now.
#[derive(Debug, Clone, Copy)]
pub struct VoteRoll<'a> {
    /// The slots that may vote.
    pub electorate: &'a [u8],
    /// Every member slot, voters or not (a kick may target a spectator).
    pub members: &'a [u8],
    /// Slots no kick vote may target (the host).
    pub protected: &'a [u8],
}

/// Why a vote message was refused. Nothing changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoteError {
    /// A vote of another kind is running.
    OtherKind,
    /// The sender is not in the electorate (joined later, a spectator, the kick's target, or a spoofed slot).
    NotEligible,
    /// The sender has voted already.
    AlreadyVoted,
    /// The sender started a vote less than 60 s ago.
    RateLimited,
    /// A kick of nobody, of oneself or of a protected slot.
    BadTarget,
    /// An option the kind does not take, or another option of a single-option vote.
    BadOption,
    /// A ninth option.
    TooManyOptions,
}

/// What a vote message or an update did.
#[derive(Debug, Clone, PartialEq)]
pub enum VoteEvent {
    /// The counts changed: send `VoteState` to every member.
    State(VoteState),
    /// The vote passed: the caller carries out `kind` with `option`.
    Passed { kind: VoteKind, option: u32 },
    /// The vote failed (time out, or no majority possible). `option` is the starter's.
    Failed { kind: VoteKind, option: u32 },
    /// The vote ended without a result (its target left, the electorate emptied, or an admin cancelled it).
    Cancelled { kind: VoteKind, option: u32 },
}

/// The server's vote box.
#[derive(Debug, Clone, Default)]
pub struct VoteBox {
    ballot: Option<Ballot>,
    starters: BTreeMap<u8, RateLimit>,
}

impl VoteBox {
    /// No vote running.
    pub fn new() -> Self {
        Self::default()
    }

    /// The running vote's counts, or `None`.
    pub fn state(&self) -> Option<VoteState> {
        self.ballot.as_ref().map(Ballot::state)
    }

    /// Handles a `Vote` from `slot`: starts a vote when none runs (with `roll` as the electorate), else casts a vote.
    pub fn submit(
        &mut self,
        slot: u8,
        vote: &Vote,
        roll: &VoteRoll<'_>,
        tick: u32,
        now_ms: u64,
    ) -> Result<Vec<VoteEvent>, VoteError> {
        let Some(ballot) = self.ballot.as_mut() else {
            return self.start(slot, vote, roll, tick, now_ms);
        };
        if ballot.kind != vote.kind {
            return Err(VoteError::OtherKind);
        }
        ballot.cast(slot, vote.option)?;
        Ok(self.settle())
    }

    /// Ends the vote at its end tick. Call every tick.
    pub fn update(&mut self, tick: u32) -> Vec<VoteEvent> {
        let Some(ballot) = self.ballot.as_ref() else {
            return Vec::new();
        };
        if tick < ballot.ends_tick {
            return Vec::new();
        }
        let failed = VoteEvent::Failed { kind: ballot.kind, option: ballot.proposal };
        self.ballot = None;
        vec![failed]
    }

    /// A member left: a kick vote against it is cancelled; a voter leaves the electorate, which can decide the vote.
    pub fn leave(&mut self, slot: u8) -> Vec<VoteEvent> {
        self.starters.remove(&slot);
        let Some(ballot) = self.ballot.as_mut() else {
            return Vec::new();
        };
        if ballot.kind == VoteKind::Kick && ballot.proposal == u32::from(slot) {
            return self.cancel();
        }
        if !ballot.remove_voter(slot) {
            return Vec::new();
        }
        if ballot.electorate.is_empty() {
            return self.cancel();
        }
        self.settle()
    }

    /// Ends the running vote without a result (an admin's cancel, a phase change). Empty when none runs.
    pub fn cancel(&mut self) -> Vec<VoteEvent> {
        let Some(ballot) = self.ballot.take() else {
            return Vec::new();
        };
        vec![VoteEvent::Cancelled { kind: ballot.kind, option: ballot.proposal }]
    }

    fn start(
        &mut self,
        slot: u8,
        vote: &Vote,
        roll: &VoteRoll<'_>,
        tick: u32,
        now_ms: u64,
    ) -> Result<Vec<VoteEvent>, VoteError> {
        if !roll.electorate.contains(&slot) {
            return Err(VoteError::NotEligible);
        }
        check_option(slot, vote, roll)?;
        let bucket = self.starters.entry(slot).or_insert_with(|| RateLimit::new(VOTE_START_RATE, now_ms));
        if !bucket.allow(now_ms) {
            return Err(VoteError::RateLimited);
        }
        let mut electorate: Vec<u8> = roll.electorate.to_vec();
        if vote.kind == VoteKind::Kick {
            electorate.retain(|voter| u32::from(*voter) != vote.option);
        }
        electorate.sort_unstable();
        electorate.dedup();
        let mut ballot = Ballot::new(vote.kind, vote.option, electorate, tick.saturating_add(VOTE_TICKS));
        ballot.cast(slot, vote.option)?;
        self.ballot = Some(ballot);
        Ok(self.settle())
    }

    /// The state after a change, and the result when the vote is decided.
    fn settle(&mut self) -> Vec<VoteEvent> {
        let Some(ballot) = self.ballot.as_ref() else {
            return Vec::new();
        };
        let mut events = vec![VoteEvent::State(ballot.state())];
        if let Some(option) = ballot.winner() {
            events.push(VoteEvent::Passed { kind: ballot.kind, option });
            self.ballot = None;
            return events;
        }
        if ballot.hopeless() {
            events.push(VoteEvent::Failed { kind: ballot.kind, option: ballot.proposal });
            self.ballot = None;
        }
        events
    }
}

/// Whether the starter's option fits the kind.
fn check_option(slot: u8, vote: &Vote, roll: &VoteRoll<'_>) -> Result<(), VoteError> {
    match vote.kind {
        VoteKind::Kick => {
            let target = u8::try_from(vote.option).ok().filter(|target| usize::from(*target) < MAX_SLOTS);
            let ok = target.is_some_and(|target| {
                target != slot && roll.members.contains(&target) && !roll.protected.contains(&target)
            });
            ok.then_some(()).ok_or(VoteError::BadTarget)
        }
        VoteKind::Restart | VoteKind::SkipResults => (vote.option == 0).then_some(()).ok_or(VoteError::BadOption),
        VoteKind::ChangeMode => is_mode(vote.option).then_some(()).ok_or(VoteError::BadOption),
        VoteKind::ChangeTrack => Ok(()),
    }
}

/// Whether `option` is a mode code a vote may propose.
pub(crate) fn is_mode(option: u32) -> bool {
    u8::try_from(option).ok().and_then(Mode::from_code).is_some()
}

/// Whether votes of `kind` may gather more options than the starter's.
pub(crate) fn many_options(kind: VoteKind) -> bool {
    matches!(kind, VoteKind::ChangeMode | VoteKind::ChangeTrack)
}

/// The most options of one vote.
pub(crate) const OPTION_LIMIT: usize = MAX_VOTE_OPTIONS;
