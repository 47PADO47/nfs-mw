//! The audio stream of a NIS scene: which take of `SOUND/STREAMS/NISAudio.big` the scene plays. Spec:
//! `docs/specs/nis-events.md` sections 7.3 (the table and its branches), 7.4 (the selectors) and 7.9 (the records,
//! the banks and the take of each scene).
//!
//! [`nis_take`] finds the scene's row in the 68-row table by its `bStringHash`, takes the branch and the scene choice of
//! that row, and asks the branch's sentence record in `NISAudio.evt` for the take that fits the choice, track 0 and the
//! complete section. The record's tests match each take's selector bits against the request's parameters, which is how
//! `ea_audio::speech::Assembler` reads it.

use blackbox_hash::bstring_hash;
use ea_audio::speech::{Assembler, Params, Random, Repeats, Sentences, SpeechIndex};

/// The table of spec section 7.3 in row order. Row 0 is `GenericStart`; rows 1 to 17 are the start branch, 18 to 35
/// the blacklist branch and 36 to 67 the end (busted) branch.
pub const NIS_SCENES: [&str; 68] = [
    "GenericStart",
    "IntroNis01",
    "IntroNis02",
    "IntroNis03",
    "IntroNis04",
    "IntroNis05",
    "IntroNis06",
    "IntroNis07",
    "IntroNis08",
    "IntroNis09",
    "IntroNis10",
    "Flythrough",
    "IntroNisDD",
    "IntroNisDDEnd",
    "IntroNisToll01",
    "GarageEnter",
    "GarageExit",
    "OPMRivalIntro",
    "IntroNisBL01",
    "IntroNisBL02",
    "IntroNisBL03",
    "IntroNisBL04",
    "IntroNisBL05",
    "IntroNisBL06",
    "IntroNisBL06End",
    "IntroNisBL07",
    "IntroNisBL07End",
    "IntroNisBL08",
    "IntroNisBL09",
    "IntroNisBL10",
    "IntroNisBL11",
    "IntroNisBL12",
    "IntroNisBL13",
    "IntroNisBL14",
    "IntroNisBL15",
    "IntroNisBL16",
    "ArrestF02",
    "ArrestF06",
    "ArrestF07",
    "ArrestF14",
    "ArrestF18",
    "ArrestF23",
    "ArrestM01",
    "ArrestM04",
    "ArrestM06",
    "ArrestM07",
    "ArrestM14",
    "ArrestM16",
    "ArrestM19",
    "ArrestM23",
    "ArrestF02b",
    "ArrestF06b",
    "ArrestF07b",
    "ArrestF14b",
    "ArrestF18b",
    "ArrestF23b",
    "ArrestM01b",
    "ArrestM04b",
    "ArrestM06b",
    "ArrestM07b",
    "ArrestM14b",
    "ArrestM16b",
    "ArrestM19b",
    "ArrestM23b",
    "EndingNis01",
    "EndingNis02",
    "EndingNis03",
    "EndingNis04",
];

/// The scene choice of each row of [`NIS_SCENES`], as the bit index of the branch's scene parameter: `NIS_Scene_Start`
/// for rows 0 to 17, `NIS_Blacklist` for 18 to 35 and `NIS_Scene_End` for 36 to 67. Spec section 7.9, step 1. [decomp]
/// and the same values in `speed.exe`. Rows that share a choice play the same take: `GarageEnter` and `GarageExit`,
/// the two blacklist end scenes and their races, and each `b` arrest and its base.
const SCENE_BITS: [u8; 68] = [
    18, // GenericStart
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, // IntroNis01 to IntroNis10
    12, 14, 19, 17, // Flythrough, IntroNisDD, IntroNisDDEnd, IntroNisToll01
    13, 13, 15, // GarageEnter, GarageExit, OPMRivalIntro
    0, 1, 2, 3, 4, 5, 5, 6, 6, // IntroNisBL01 to IntroNisBL05, IntroNisBL06 and its End, IntroNisBL07 and its End
    7, 8, 9, 10, 11, 12, 13, 14, 15, // IntroNisBL08 to IntroNisBL16
    7, 8, 9, 10, 14, 11, // ArrestF02, F06, F07, F14, F18, F23
    0, 1, 2, 15, 3, 4, 5, 6, // ArrestM01, M04, M06, M07, M14, M16, M19, M23
    7, 8, 9, 10, 14, 11, // ArrestF02b to ArrestF23b
    0, 1, 2, 15, 3, 4, 5, 6, // ArrestM01b to ArrestM23b
    17, 18, 19, 20, // EndingNis01 to EndingNis04
];

/// The section of a normal play: `Complete`, bit 0 of `NIS_Section`. A skip asks for `End`, bit 2. [decomp]
const COMPLETE: u32 = 1 << 0;

/// The track every row asks for: `Track00`, bit 13 of `NIS_Track`. [decomp] and `speed.exe`. The original asks
/// `GenericStart` for the scene's camera track instead; the request does not carry it, so it asks for track 0 too.
const TRACK_00: u32 = 1 << 13;

/// Which selector the scene plays from. Spec section 7.3, step 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NisBranch {
    /// Rows 0 to 17: `NIS_Select_Start`.
    Start,
    /// Rows 18 to 35: `NIS_Select_Blacklist`.
    Blacklist,
    /// Rows 36 to 67: `NIS_Select_End`, with the busted stream.
    End,
}

impl NisBranch {
    /// The branch of table row `row`, or `None` past the table.
    pub fn of_row(row: usize) -> Option<Self> {
        match row {
            0..=17 => Some(Self::Start),
            18..=35 => Some(Self::Blacklist),
            36..=67 => Some(Self::End),
            _ => None,
        }
    }

    /// The event number of the branch's record in `NISAudio.evt`. [verified] The records lie in the order of the
    /// interface list stored after them (start, sound effect, blacklist, end), and each record's three tests have the
    /// choice counts of its interface's parameters: 20, 20 and 3 for the start record, 19, 20 and 3 for the blacklist
    /// record and 21, 20 and 3 for the end record (spec section 7.9, step 3).
    fn event(self) -> u16 {
        match self {
            Self::Start => 0x4000,
            Self::Blacklist => 0x4014,
            Self::End => 0x4011,
        }
    }

    /// The argument numbers of the scene, the track and the section in the branch's interface, in that order. [decomp]
    /// and the record's tests: the blacklist interface takes the track and the section first.
    fn arguments(self) -> (u8, u8, u8) {
        match self {
            Self::Start | Self::End => (1, 2, 3),
            Self::Blacklist => (3, 1, 2),
        }
    }

    /// The parameters of a normal play of the scene with choice `bit`: that scene, track 0 and the complete section.
    fn params(self, bit: u8) -> Params {
        let (scene, track, section) = self.arguments();
        Params::new().with(scene, 1 << bit).with(track, TRACK_00).with(section, COMPLETE)
    }
}

/// One take of the NIS audio: `bank` is the bank's position in [`SpeechIndex::banks`] and `take` the take in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NisTake {
    pub bank: usize,
    pub take: usize,
}

/// The table row of a scene, found by the `bStringHash` of its name. `None` for a scene outside the table: those play
/// no NIS stream (the FMVs, replays and moments).
pub fn nis_row(scene: &str) -> Option<usize> {
    let hash = bstring_hash(scene);
    NIS_SCENES.iter().position(|name| bstring_hash(name) == hash)
}

/// The branch of a scene, or `None` for a scene outside the table.
pub fn nis_branch(scene: &str) -> Option<NisBranch> {
    NisBranch::of_row(nis_row(scene)?)
}

/// The take a NIS scene plays: the take of its branch's bank that the branch's record allows for the scene's choice,
/// track 0 and the complete section (spec section 7.9). Each bank has one such take per choice.
///
/// `None` for a scene outside the table, and for a table scene the install has no take for (`Flythrough`,
/// `GarageEnter`, `GarageExit`, `OPMRivalIntro` and `IntroNisBL01`): those play no stream. [unconfirmed] What the
/// original plays when its selection finds no take is not known; another scene's take would be wrong, so none plays.
pub fn nis_take(index: &SpeechIndex, sentences: &Sentences, scene: &str) -> Option<NisTake> {
    let row = nis_row(scene)?;
    let branch = NisBranch::of_row(row)?;
    let params = branch.params(SCENE_BITS[row]);
    let assembler = Assembler::new(sentences, index);
    let chosen = assembler.assemble(branch.event(), &params, &mut Repeats::default(), &mut First)?;
    let first = chosen.first()?;
    Some(NisTake { bank: first.bank, take: first.take })
}

/// A choice source that always takes the first option, so a NIS scene always plays the same take.
struct First;

impl Random for First {
    fn below(&mut self, _n: usize) -> usize {
        0
    }
}

#[cfg(test)]
#[path = "nis_tests.rs"]
mod tests;
