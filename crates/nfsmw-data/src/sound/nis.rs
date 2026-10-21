//! The audio stream of a NIS scene: which take of `SOUND/STREAMS/NISAudio.big` the scene plays. Spec:
//! `docs/specs/nis-events.md` sections 7.3 (the table and its branches), 7.4 (the selectors) and 7.8 (the lookup).
//!
//! [`nis_take`] finds the scene's row in the 68-row table by its `bStringHash`, takes the branch of that row, and asks
//! the sentence database (`NISAudio.evt`) for the take of that branch. The sentence database holds one record per
//! branch; the record's tests match each take's selector bits against the request's parameters, which is how
//! `ea_audio::speech::Assembler` reads it.

use blackbox_hash::bstring_hash;
use ea_audio::speech::{Assembler, Chosen, Params, Random, Repeats, Sentences, SpeechIndex};

/// The table of spec section 7.3 in row order. Row 0 is `GenericStart`; rows 1 to 17 are the start branch, 18 to 35
/// the blacklist branch and 36 to 67 the end (busted) branch. Rows 1 to 17 are in the order the spec gives, and the
/// start branch's takes carry those rows as their scene bits.
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
    "IntroNisBL07",
    "IntroNisBL08",
    "IntroNisBL09",
    "IntroNisBL10",
    "IntroNisBL11",
    "IntroNisBL12",
    "IntroNisBL13",
    "IntroNisBL14",
    "IntroNisBL15",
    "IntroNisBL16",
    "IntroNisBL06End",
    "IntroNisBL07End",
    "ArrestF02",
    "ArrestF02b",
    "ArrestF06",
    "ArrestF06b",
    "ArrestF07",
    "ArrestF07b",
    "ArrestF14",
    "ArrestF14b",
    "ArrestF18",
    "ArrestF18b",
    "ArrestF23",
    "ArrestF23b",
    "ArrestM01",
    "ArrestM01b",
    "ArrestM04",
    "ArrestM04b",
    "ArrestM06",
    "ArrestM06b",
    "ArrestM07",
    "ArrestM07b",
    "ArrestM14",
    "ArrestM14b",
    "ArrestM16",
    "ArrestM16b",
    "ArrestM19",
    "ArrestM19b",
    "ArrestM23",
    "ArrestM23b",
    "EndingNis01",
    "EndingNis02",
    "EndingNis03",
    "EndingNis04",
];

/// The section value of a normal play: the complete section, which the takes carry as selector bit 0. [inferred]
/// The spec names the section but not its number; the takes of the start and end branches use bits 0 and 2, and the
/// skip (end) section is 2.
const COMPLETE: u32 = 1 << 0;

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

    /// The event number of the branch's record in `NISAudio.evt`. [inferred] Each NIS record names one bank by its
    /// phrase: the start record the bank of 34 takes (17 scene bits, each with the complete and the skip section), the
    /// blacklist record the bank of 18 takes and the end record the bank of 32, one per row. The `.csi` names are not
    /// stored beside the events, so this link is read off the install's data, not from the spec.
    fn event(self) -> u16 {
        match self {
            Self::Start => 0x4014,
            Self::Blacklist => 0x4011,
            Self::End => 0x4000,
        }
    }

    /// The parameter number of the record's section test. [verified from the records' tests]
    fn section_param(self) -> u8 {
        match self {
            Self::Start => 2,
            Self::Blacklist | Self::End => 3,
        }
    }

    /// The parameter number of the record's scene test. [verified from the records' tests]
    fn scene_param(self) -> u8 {
        match self {
            Self::Start => 3,
            Self::Blacklist | Self::End => 1,
        }
    }

    /// The bit a scene's take carries in its first selector. [unconfirmed] For the start branch the takes carry exactly
    /// the bits 1 to 17, one per row, so the row is the bit. The blacklist and end branches have 18 and 32 rows for
    /// 18 and 15 distinct first-selector values, and the spec does not give their mapping: they have no bit.
    fn scene_bit(self, row: usize) -> Option<u32> {
        match self {
            Self::Start => u32::try_from(row).ok(),
            Self::Blacklist | Self::End => None,
        }
    }
}

/// One take of the NIS audio: `bank` is the bank's position in [`SpeechIndex::banks`] and `take` the take in it.
/// `exact` is false when the scene's own take could not be picked, and the take is the first complete take of the
/// scene's branch instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NisTake {
    pub bank: usize,
    pub take: usize,
    pub exact: bool,
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

/// The take a NIS scene plays. Spec section 7.3 gives the table and the branch; the take is picked by the branch's
/// sentence record. [unconfirmed] The spec does not say how a parameter set picks one take (section 7.4), so:
///
/// - a start scene asks for its own scene bit and the complete section, and takes the one take that fits;
/// - a blacklist or end scene has no known scene bit, so it takes the first complete take of its branch, and
///   [`NisTake::exact`] is false. This is the rule the spec's "cannot pick a single take" case gives;
/// - a start scene with no take for its bit (`GenericStart`, bit 0) also takes the first complete take of its branch.
///
/// The section is the complete one, which is normal play. The skip (end) section is not chosen here.
pub fn nis_take(index: &SpeechIndex, sentences: &Sentences, scene: &str) -> Option<NisTake> {
    let row = nis_row(scene)?;
    let branch = NisBranch::of_row(row)?;
    let assembler = Assembler::new(sentences, index);
    let section = Params::new().with(branch.section_param(), COMPLETE);
    let exact = branch.scene_bit(row).and_then(|bit| {
        let params = section.with(branch.scene_param(), 1 << bit);
        pick(&assembler, branch.event(), &params)
    });
    if let Some(chosen) = exact {
        return Some(NisTake { bank: chosen.bank, take: chosen.take, exact: true });
    }
    let chosen = pick(&assembler, branch.event(), &section)?;
    Some(NisTake { bank: chosen.bank, take: chosen.take, exact: false })
}

/// The first take the sentence record of `event` makes for `params`. The record has one block of weight 100, so the
/// block is always tried; the random choice is fixed to the first fitting take.
fn pick(assembler: &Assembler, event: u16, params: &Params) -> Option<Chosen> {
    let chosen = assembler.assemble(event, params, &mut Repeats::default(), &mut First)?;
    chosen.first().copied()
}

/// A choice source that always takes the first option, so a NIS scene always plays the same take.
struct First;

impl Random for First {
    fn below(&mut self, _n: usize) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_table_has_the_spec_rows_and_no_two_scenes_share_a_hash() {
        assert_eq!(NIS_SCENES.len(), 68);
        assert_eq!(NIS_SCENES[0], "GenericStart");
        assert_eq!(NIS_SCENES[17], "OPMRivalIntro");
        assert_eq!(NIS_SCENES[18], "IntroNisBL01");
        assert_eq!(NIS_SCENES[35], "IntroNisBL07End");
        assert_eq!(NIS_SCENES[36], "ArrestF02");
        assert_eq!(NIS_SCENES[67], "EndingNis04");
        let hashes: HashSet<u32> = NIS_SCENES.iter().map(|name| bstring_hash(name)).collect();
        assert_eq!(hashes.len(), 68);
    }

    #[test]
    fn a_scene_takes_the_branch_of_its_row() {
        assert_eq!(nis_branch("GenericStart"), Some(NisBranch::Start));
        assert_eq!(nis_branch("IntroNis05"), Some(NisBranch::Start));
        assert_eq!(nis_branch("Flythrough"), Some(NisBranch::Start));
        assert_eq!(nis_branch("IntroNisBL02"), Some(NisBranch::Blacklist));
        assert_eq!(nis_branch("IntroNisBL06End"), Some(NisBranch::Blacklist));
        assert_eq!(nis_branch("ArrestF02"), Some(NisBranch::End));
        assert_eq!(nis_branch("ArrestM23b"), Some(NisBranch::End));
        assert_eq!(nis_branch("EndingNis04"), Some(NisBranch::End));
    }

    #[test]
    fn a_scene_outside_the_table_plays_no_stream() {
        assert_eq!(nis_row("FMVTest"), None);
        assert_eq!(nis_row("ReplayBurnout"), None);
        assert_eq!(nis_branch("WMBoat"), None);
    }

    #[test]
    fn only_the_start_branch_has_a_scene_bit() {
        assert_eq!(NisBranch::Start.scene_bit(5), Some(5));
        assert_eq!(NisBranch::Start.scene_bit(0), Some(0));
        assert_eq!(NisBranch::Blacklist.scene_bit(20), None);
        assert_eq!(NisBranch::End.scene_bit(40), None);
    }

    #[test]
    fn the_first_choice_is_the_first_option() {
        assert_eq!(First.below(7), 0);
        assert_eq!(First.below(0), 0);
    }
}
