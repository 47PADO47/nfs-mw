//! The messages each Blacklist bin sends (`docs/specs/career-scripts.md` section 3.3, **[verified]** from the bin
//! scripts' bytecode). Sender and text rules are in `docs/specs/career-phone.md`.
//!
//! A bin sends a message at four moments: when it is entered, when its rival unlocks, and at two counted points,
//! a race-win count and a challenge count. [`bin_messages`] returns the ids for one moment. World heat, which the
//! table lists beside a few race wins (bins 7 and 2), is not a message and is not here.

/// When a bin's message is sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinMoment {
    /// The bin is entered: its entry message (`EntryCellCallID`).
    Entry,
    /// The bin's rival unlocks: its rival message (`SMSRivalChallenge`).
    RivalUnlock,
    /// The bin's races-won counter equals `n` right after a first place.
    RaceWins(u8),
    /// The bin's challenges-passed counter reaches `n`. Each message is sent once.
    Challenges(u8),
}

/// One trigger per row: the bin, the moment and the ids sent then.
const TABLE: &[(u8, BinMoment, &[u32])] = &[
    // Entry messages (bin 15 has none; bin 14 sends two at the start of the bin).
    (14, BinMoment::Entry, &[125, 126]),
    (13, BinMoment::Entry, &[30]),
    (12, BinMoment::Entry, &[37]),
    (11, BinMoment::Entry, &[42]),
    (10, BinMoment::Entry, &[48]),
    (9, BinMoment::Entry, &[54]),
    (8, BinMoment::Entry, &[58]),
    (7, BinMoment::Entry, &[64]),
    (6, BinMoment::Entry, &[70]),
    (5, BinMoment::Entry, &[73]),
    (4, BinMoment::Entry, &[77]),
    (3, BinMoment::Entry, &[80]),
    (2, BinMoment::Entry, &[85]),
    (1, BinMoment::Entry, &[89]),
    // Rival unlock messages (bin 1 sends two, the script copy).
    (15, BinMoment::RivalUnlock, &[20]),
    (14, BinMoment::RivalUnlock, &[29]),
    (13, BinMoment::RivalUnlock, &[36]),
    (12, BinMoment::RivalUnlock, &[41]),
    (11, BinMoment::RivalUnlock, &[46]),
    (10, BinMoment::RivalUnlock, &[52]),
    (9, BinMoment::RivalUnlock, &[57]),
    (8, BinMoment::RivalUnlock, &[63]),
    (7, BinMoment::RivalUnlock, &[69]),
    (6, BinMoment::RivalUnlock, &[72]),
    (5, BinMoment::RivalUnlock, &[76]),
    (4, BinMoment::RivalUnlock, &[79]),
    (3, BinMoment::RivalUnlock, &[84]),
    (2, BinMoment::RivalUnlock, &[88]),
    (1, BinMoment::RivalUnlock, &[93, 94]),
    // Races-won messages (on the exact count).
    (15, BinMoment::RaceWins(1), &[123]),
    (14, BinMoment::RaceWins(1), &[18]),
    (14, BinMoment::RaceWins(2), &[27]),
    (13, BinMoment::RaceWins(1), &[23]),
    (13, BinMoment::RaceWins(2), &[24]),
    (13, BinMoment::RaceWins(4), &[31]),
    (12, BinMoment::RaceWins(1), &[127]),
    (12, BinMoment::RaceWins(2), &[17]),
    (12, BinMoment::RaceWins(4), &[40]),
    (11, BinMoment::RaceWins(2), &[128]),
    (11, BinMoment::RaceWins(4), &[130]),
    (10, BinMoment::RaceWins(3), &[49]),
    (9, BinMoment::RaceWins(2), &[53]),
    (9, BinMoment::RaceWins(5), &[55]),
    (8, BinMoment::RaceWins(2), &[59]),
    (8, BinMoment::RaceWins(4), &[60]),
    (7, BinMoment::RaceWins(1), &[66]),
    (7, BinMoment::RaceWins(4), &[65]),
    (7, BinMoment::RaceWins(7), &[134]),
    (6, BinMoment::RaceWins(3), &[135]),
    (4, BinMoment::RaceWins(4), &[136]),
    (3, BinMoment::RaceWins(2), &[81]),
    (3, BinMoment::RaceWins(4), &[82]),
    (2, BinMoment::RaceWins(4), &[86]),
    (1, BinMoment::RaceWins(4), &[90]),
    (1, BinMoment::RaceWins(6), &[91]),
    // Challenges messages (on reaching the count).
    (15, BinMoment::Challenges(1), &[124]),
    (15, BinMoment::Challenges(3), &[137]),
    (14, BinMoment::Challenges(2), &[19]),
    (14, BinMoment::Challenges(3), &[22]),
    (13, BinMoment::Challenges(1), &[35]),
    (13, BinMoment::Challenges(2), &[25]),
    (13, BinMoment::Challenges(3), &[26]),
    (12, BinMoment::Challenges(1), &[33]),
    (11, BinMoment::Challenges(2), &[38]),
    (10, BinMoment::Challenges(2), &[132]),
    (9, BinMoment::Challenges(2), &[43]),
    (8, BinMoment::Challenges(3), &[61]),
    (7, BinMoment::Challenges(2), &[133]),
    (6, BinMoment::Challenges(2), &[100]),
    (5, BinMoment::Challenges(3), &[101]),
    (4, BinMoment::Challenges(4), &[102]),
];

/// The message ids a bin sends at `moment`, in the order the script queues them. Empty when the bin sends nothing
/// then (for example bin 15 on entry, or bin 5 on a race win).
pub fn bin_messages(bin: u8, moment: BinMoment) -> &'static [u32] {
    let row = TABLE.iter().find(|(row_bin, row_moment, _)| *row_bin == bin && *row_moment == moment);
    match row {
        Some((_, _, ids)) => ids,
        None => &[],
    }
}

#[cfg(test)]
#[path = "bins_tests.rs"]
mod tests;
