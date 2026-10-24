//! The overview's pure rules: which rival is shown, the panel of a rival, the sheets and the Remastered list's entries.

use nfsmw_career::blacklist::Counters;
use nfsmw_career::settings::{FIRST_RIVAL_STAGE, Flags};

use super::super::blacklist::{RivalRow, start_bin};
use crate::frontend::ids::screen;

/// Lowest rival the overview steps to: the one the career is at (the earlier ones are the beaten rivals above it).
pub fn lowest(current: u8) -> u8 {
    start_bin(current)
}

/// The rival shown when the page opens: the one asked for, kept between the lowest and rival 15; none asked is the lowest.
pub fn first_shown(asked: u8, low: u8) -> u8 {
    if asked == 0 {
        return low;
    }
    asked.clamp(low, FIRST_RIVAL_STAGE)
}

/// One step of Up or Down: Up goes to the next higher rival and wraps from 15 to
/// the lowest, Down goes to the next lower and wraps from the lowest to 15.
pub fn step(shown: u8, low: u8, up: bool) -> u8 {
    if up {
        let next = shown.saturating_add(1);
        if next > FIRST_RIVAL_STAGE {
            return low;
        }
        return next;
    }
    let next = shown.saturating_sub(1);
    if next < low {
        return FIRST_RIVAL_STAGE;
    }
    next
}

/// One row of the panel: what the player has against what the rival asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub have: u64,
    pub need: u32,
}

impl Requirement {
    /// Met when the player has at least what is asked (the panel's tick).
    pub fn met(self) -> bool {
        self.have >= u64::from(self.need)
    }
}

/// The Challenge Rival button: gone for a beaten rival, otherwise on or off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Hidden,
    Off,
    On,
}

/// What the page shows of one rival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub bin: u8,
    pub races: Requirement,
    pub challenges: Requirement,
    pub bounty: Requirement,
    /// A beaten rival: the stamp shows and the button is gone.
    pub defeated: bool,
    pub button: Button,
    pub cash: u32,
}

/// The panel of `row` while the career is at `current`: a rival above the career's bin is beaten, and so is every
/// rival once the career is complete.
pub fn panel(row: &RivalRow, current: u8, flags: Flags, bounty: u64, cash: u32) -> Panel {
    let Counters { races_won, challenges } = row.counters;
    let defeated = flags.contains(Flags::CAREER_COMPLETE) || row.bin > current;
    let button = match (defeated, row.button_active) {
        (true, _) => Button::Hidden,
        (false, true) => Button::On,
        (false, false) => Button::Off,
    };
    Panel {
        bin: row.bin,
        races: Requirement { have: u64::from(races_won), need: row.races_needed },
        challenges: Requirement { have: u64::from(challenges), need: row.challenges_needed },
        bounty: Requirement { have: bounty, need: row.bounty_needed },
        defeated,
        button,
        cash,
    }
}

/// The four sheets the page offers, in the order Left and Right walk them (the original's option ids 0, 1, 2, 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sheet {
    RaceEvents,
    Milestones,
    Bounty,
    Bio,
}

pub const SHEETS: [Sheet; 4] = [Sheet::RaceEvents, Sheet::Milestones, Sheet::Bounty, Sheet::Bio];

impl Sheet {
    /// The label of the sheet's name (`REP_SHEET_RACE_EVENTS`, `REP_SHEET_MILESTONE_EVENTS`, `REP_SHEET_BOUNTY`,
    /// `REP_SHEET_BIO`).
    pub fn label(self) -> u32 {
        match self {
            Sheet::RaceEvents => 0x84E4_A54C,
            Sheet::Milestones => 0x216F_1B81,
            Sheet::Bounty => 0xE451_941E,
            Sheet::Bio => 0x2D15_9737,
        }
    }

    /// The screen the sheet opens.
    pub fn screen(self) -> Option<&'static str> {
        match self {
            Sheet::RaceEvents => Some(screen::SAFE_HOUSE_RACE_SHEET),
            Sheet::Milestones => Some(screen::SAFE_HOUSE_MILESTONES),
            Sheet::Bounty => Some(screen::SAFE_HOUSE_BOUNTY),
            Sheet::Bio => Some(screen::SAFE_HOUSE_RIVAL_BIO),
        }
    }
}

/// One entry of the Remastered look's list for a rival: a sheet, or the Challenge Rival button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Sheet(Sheet),
    Challenge,
}

/// The Remastered look has no overview page: the rival shows as a list of the four sheets, then the Challenge Rival
/// button unless the rival is beaten (it is greyed while the button is off). Same rules as the Original page's button.
pub fn entries(button: Button) -> Vec<(Entry, bool)> {
    let mut list: Vec<(Entry, bool)> = SHEETS.into_iter().map(|sheet| (Entry::Sheet(sheet), true)).collect();
    if button != Button::Hidden {
        list.push((Entry::Challenge, button == Button::On));
    }
    list
}
