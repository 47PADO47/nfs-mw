//! Which solids of the 2005 world the 2012 trees replace, and with which species. The names are the game's own (the solids of its
//! tree scenery, `docs/formats/chameleon-vegetation.md`); the mod does not read the 2005 install, so they are listed here. A
//! solid the game does not have is a swap the game never uses.
//!
//! A *single* tree is replaced by one model scaled to its height; a *row* (a tree line, a canopy, a clump, a filler) by copies
//! of a model along the long side of its bounds (`blackbox_modding::content::SwapFit`). The levels of detail of a solid are
//! the letter after its number (`1B`, `1C`, `1Z`): the finest letters take the finest level of the model, `C` the next, `Z` the
//! coarsest.

/// A stem of the game's solid names with the levels the game has of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    /// The name up to the number: `XT_CEDARM`.
    pub stem: &'static str,
    /// What follows the level code: `JG_00`.
    pub tail: &'static str,
    /// The level codes: a digit and the letter, `1B`.
    pub codes: &'static [&'static str],
    /// The key of the species ([`super::species::SPECIES`]).
    pub species: &'static str,
    /// A stretch of trees rather than one tree.
    pub row: bool,
}

impl Family {
    /// The names of the solids of the family, each with its level code.
    pub fn names(&self) -> impl Iterator<Item = (String, &'static str)> + '_ {
        self.codes.iter().map(|&code| (format!("{}_{code}_{}", self.stem, self.tail), code))
    }
}

const fn single(
    stem: &'static str,
    tail: &'static str,
    codes: &'static [&'static str],
    species: &'static str,
) -> Family {
    Family { stem, tail, codes, species, row: false }
}

const fn row(stem: &'static str, tail: &'static str, codes: &'static [&'static str], species: &'static str) -> Family {
    Family { stem, tail, codes, species, row: true }
}

const BCZ: &[&str] = &["1B", "1C", "1Z"];
const ACZ: &[&str] = &["1A", "1C", "1Z"];
const BZ: &[&str] = &["1B", "1Z"];
const ABCZ: &[&str] = &["1A", "1B", "1C", "1Z"];

/// Every family the variant replaces.
pub const FAMILIES: &[Family] = &[
    single("XT_CEDARM", "JG_00", BCZ, "CONIFER_TALL"),
    single("XT_CEDARS", "JG_00", ACZ, "SPRUCE_S"),
    single("XT_SPRUCEM", "JG_00", ACZ, "SPRUCE"),
    single("XT_CT_POPLARD", "JG_00", BCZ, "POPLAR"),
    single("XT_CT_POPLARS", "JG_00", BCZ, "POPLAR"),
    single("XO_CT_CYPRESS", "00", &["1B", "1C"], "POPLAR"),
    single("XT_CT_FALLTREED", "JG_00", BCZ, "AUTUMN"),
    single("XT_CT_FALLTREE2D", "JG_00", BCZ, "AUTUMN"),
    single("XT_CT_FALLTREES", "JG_00", BCZ, "AUTUMN"),
    single("XT_JPNMAPLE", "JG_00", BCZ, "AUTUMN"),
    single("XT_CT_FALLTREEDEAD", "JG_00", BCZ, "DEAD"),
    single("XT_CT_FALLTREEDEAD2", "JG_00", BCZ, "DEAD"),
    single("XT_CT_FALLTREEDEAD3", "JG_00", BCZ, "DEAD"),
    single("XT_TREEMAPLEX", "LL_00", BCZ, "BROADLEAF_L"),
    single("XO_CT_CAMPUSSMACKTREE", "JG_00", BCZ, "BROADLEAF_S"),
    single("XO_CT_CAMPUSSMACKTREEB", "JG_00", BCZ, "BROADLEAF_S"),
    single("XT_BUSHMULLD", "RB_00", &["1B", "1Z", "2B", "2Z"], "BUSH"),
    single("XT_BUSHYELLOWFLOWERS", "RB_00", &["1B", "1Z", "2B", "2Z"], "BUSH"),
    row("XT_BUSHGROUP", "00", BZ, "BUSH"),
    row("XT_CT_TREELINE1", "00", BZ, "BROADLEAF_M"),
    row("XT_CT_TREELINE1Z", "00", ABCZ, "BROADLEAF_M"),
    row("XT_CT_TREELINE1MED", "00", &["1B"], "BROADLEAF_M"),
    row("XT_EVRGRN_TREELINEL", "00", BZ, "SPRUCE_S"),
    row("XT_EVRGRN_TREELINES", "00", &["1B"], "SPRUCE_S"),
    row("XT_CT_FALLTREEFILLERA", "00", BZ, "AUTUMN_THIN"),
    row("XT_CT_FALLTREEFILLERB", "00", BZ, "AUTUMN_THIN"),
    row("XT_CT_JUNIPERROWA", "00", &["1B", "1C"], "POPLAR"),
    row("XO_CT_CYPRESSCLUMP", "00", ABCZ, "POPLAR"),
    row("XT_CT_TREECANOPY", "00", ABCZ, "BROADLEAF_L"),
];

/// The level of a model with `levels` levels that the solid with level code `code` takes: the finest letters (`A`, `B`) the
/// finest level of a single tree and the second of a row (a row holds many copies, so each is coarser), `C` the next, and every
/// other letter (`Z`) the coarsest.
pub fn level(code: &str, row: bool, levels: usize) -> usize {
    let wanted = match (code.chars().nth(1), row) {
        (Some('A' | 'B'), false) => 0,
        (Some('A' | 'B'), true) | (Some('C'), false) => 1,
        (Some('C'), true) => 2,
        _ => usize::MAX,
    };
    wanted.min(levels.saturating_sub(1))
}
