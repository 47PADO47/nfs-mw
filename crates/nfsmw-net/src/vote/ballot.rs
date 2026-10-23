//! One running vote: the electorate, the options and who voted for each.

use super::{OPTION_LIMIT, VoteError, is_mode, many_options};
use crate::message::{List, VoteKind, VoteOption, VoteState};

#[derive(Debug, Clone)]
pub(super) struct Ballot {
    pub kind: VoteKind,
    /// The starter's option (the kick target's slot for a kick).
    pub proposal: u32,
    /// The slots that may vote, sorted.
    pub electorate: Vec<u8>,
    /// Each option and its voters, in the order the options were proposed.
    options: Vec<(u32, Vec<u8>)>,
    pub ends_tick: u32,
}

impl Ballot {
    pub fn new(kind: VoteKind, proposal: u32, electorate: Vec<u8>, ends_tick: u32) -> Self {
        Self { kind, proposal, electorate, options: vec![(proposal, Vec::new())], ends_tick }
    }

    /// Counts `slot`'s vote for `option`.
    pub fn cast(&mut self, slot: u8, option: u32) -> Result<(), VoteError> {
        if self.electorate.binary_search(&slot).is_err() {
            return Err(VoteError::NotEligible);
        }
        if self.options.iter().any(|(_, voters)| voters.contains(&slot)) {
            return Err(VoteError::AlreadyVoted);
        }
        if let Some((_, voters)) = self.options.iter_mut().find(|(id, _)| *id == option) {
            voters.push(slot);
            return Ok(());
        }
        if !many_options(self.kind) || (self.kind == VoteKind::ChangeMode && !is_mode(option)) {
            return Err(VoteError::BadOption);
        }
        if self.options.len() >= OPTION_LIMIT {
            return Err(VoteError::TooManyOptions);
        }
        self.options.push((option, vec![slot]));
        Ok(())
    }

    /// Takes `slot` out of the electorate and its vote out of the counts. `false` when it was not a voter.
    pub fn remove_voter(&mut self, slot: u8) -> bool {
        let Ok(at) = self.electorate.binary_search(&slot) else {
            return false;
        };
        self.electorate.remove(at);
        for (_, voters) in &mut self.options {
            voters.retain(|voter| *voter != slot);
        }
        true
    }

    /// The option more than half of the electorate voted for, if any.
    pub fn winner(&self) -> Option<u32> {
        let needed = self.electorate.len() / 2 + 1;
        self.options.iter().find(|(_, voters)| voters.len() >= needed).map(|(id, _)| *id)
    }

    /// Whether no option can reach a majority any more, even if every voter left votes for the best one.
    pub fn hopeless(&self) -> bool {
        let needed = self.electorate.len() / 2 + 1;
        let voted: usize = self.options.iter().map(|(_, voters)| voters.len()).sum();
        let open = self.electorate.len().saturating_sub(voted);
        let best = self.options.iter().map(|(_, voters)| voters.len()).max().unwrap_or(0);
        best + open < needed
    }

    pub fn state(&self) -> VoteState {
        let counts = self
            .options
            .iter()
            .map(|(id, voters)| VoteOption { id: *id, count: u8::try_from(voters.len()).unwrap_or(u8::MAX) })
            .collect();
        VoteState { kind: self.kind, options: List::new(counts).unwrap_or_default(), ends_tick: self.ends_tick }
    }
}
