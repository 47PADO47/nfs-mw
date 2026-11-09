//! The bushes, hedges and grasses of the 2005 world and the 2012 models that stand in for them: the **bushes set**, a value of
//! the game's own option "Bushes" (`bushes_variant`) that the mod declares through the replacement API (`replace/bushes-mw2012.toml`,
//! `docs/modding/api-replace.md`), next to the trees set. What the 2012 models are and how the pairs were chosen:
//! `docs/formats/chameleon-vegetation.md`, "The bushes".
//!
//! The 2005 low growth is mostly **patches**: a solid that is a world-space copy of a stretch of bushes, 30 to 100 m long with a
//! few hundred vertices, on a slope, in a forest or along a road. A patch is replaced by copies of one small model, one for each
//! stretch of its old vertices (`fit = "row"`), so a patch of bushes stays a patch of bushes (`effect-variants.md`).

use super::manifest::{Limits, Set};
use super::stock::{BCZ, BZ, Family, row, single};

/// The limits of the bushes: a bush may be wider than the cell it stands in (a patch is dense), squeezed sideways far less than a tree,
/// and a patch whose old cells are too thin for a bush stays as it was. `clearance`: a bush never grows into the guard rail, barrier, wall or
/// building beside it (`docs/modding/api-replace.md`, "Clearance").
pub const BUSH_LIMITS: Limits = Limits { spread: 1.5, squeeze: 0.5, min_height: 0.5, clearance: true };
/// The message of the mod's catalogue that names the option.
pub const KIND_LABEL: &str = "nfsmw2012-bushes";

const A2: &[&str] = &["1A", "2A"];
const B1: &[&str] = &["1B"];

/// Every family the bushes set replaces: the bushes of the old trees value (`stock::LEGACY_BUSHES`), the bush patches, the hedges and
/// the grasses.
pub const FAMILIES: &[Family] = &[
    // Bush patches.
    row("XT_BUSHGROUP", "00", BZ, "BUSH"),
    single("XT_BUSHMULLD", "RB_00", &["1B", "1Z", "2B", "2Z"], "BUSH"),
    single("XT_BUSHYELLOWFLOWERS", "RB_00", &["1B", "1Z", "2B", "2Z"], "PALE_SHRUB"),
    row("XT_CT_BUSH", "JG_00", BZ, "SHRUB_GREEN"),
    row("XT_BUSH_FOREST", "JG_00", A2, "DARK_SHRUB"),
    row("XT_GC_BUSH", "00", B1, "SHRUB_STRIP"),
    row("XT_CT_BUSHMULLB", "00", BZ, "BUSH"),
    row("XT_BUSHREDA", "LL_00", BZ, "DARK_SHRUB"),
    row("XT_BUSHB", "AL_00", &["1C"], "SHRUB_GREEN"),
    // Hedges.
    row("XT_HEDGE", "00", BZ, "HEDGE"),
    row("XT_HEDGE2", "00", BZ, "HEDGE"),
    row("XT_HEDGE3", "00", BZ, "HEDGE"),
    row("XT_HEDGEA4M", "00", BZ, "HEDGE"),
    row("XT_HEDGEA8M", "00", BZ, "HEDGE"),
    row("XT_HEDGECURVEDA", "JM_00", BZ, "HEDGE"),
    row("XT_HEDGEMUL_A", "AP_00", BCZ, "HEDGE"),
    row("XT_HEDGEMUL_B", "AP_00", BCZ, "HEDGE"),
    row("XT_HEDGEMUL_C", "AP_00", BCZ, "HEDGE"),
    row("XT_HEDGEMUL_D", "AP_00", B1, "HEDGE"),
    row("XT_HEDGEMUL_E", "AP_00", BZ, "HEDGE"),
    row("XW_HEDGESTADIUM", "00", BZ, "HEDGE"),
    // Grasses.
    row("XO_GRASSCLUMPCT", "JG_00", &["1B", "2B"], "GRASS_CLUMP"),
    row("XT_BEACHGRASS", "00", BZ, "GRASS_TALL"),
    row("XT_GRASSA", "LL_00", BZ, "GRASS_TALL"),
    row("XT_GRASSB", "LL_00", BZ, "GRASS_TALL"),
    row("XT_GRASSC", "LL_00", BZ, "GRASS_TALL"),
    row("XT_GRASSPATCH", "00", BZ, "GRASS_TALL"),
];

/// The set the mod declares for the bushes.
pub fn set() -> Set<'static> {
    Set {
        kind: "bushes",
        kind_label: KIND_LABEL,
        value: super::VARIANT,
        value_label: super::LABEL,
        limits: BUSH_LIMITS,
        families: FAMILIES.iter().collect(),
    }
}
