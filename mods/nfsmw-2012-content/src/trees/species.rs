//! The trees of the 2012 world the variant carries. Each is one model of the world's bundles (`HAWAII/`): a level-of-detail
//! set whose finest levels are a trunk and cards of leaves drawn with the `Tree_Translucent_1Bit_Doublesided` technique. The
//! world holds the same model in every bundle (district or track unit) that uses it; the bundle named here is one that holds it
//! together with all it needs but the shared resources of `GLOBALRESOURCES.BNDL` and `SHADERS.BNDL`
//! (`docs/formats/chameleon-vegetation.md`).

/// One tree of the 2012 world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Species {
    /// What the pack's solids and the profile call it: the solids are `MW12_<KEY>_L<level>`.
    pub key: &'static str,
    /// The id of its model.
    pub model: u64,
    /// The bundle of `HAWAII/` it is read from.
    pub bundle: &'static str,
    /// What it is, for the log.
    pub label: &'static str,
}

/// The bundle that holds the resources the models share (the leaf atlas and the bark of several among them).
pub const GLOBAL_BUNDLE: &str = "HAWAII/GLOBALRESOURCES.BNDL";

/// The species, in the order the pack lists them.
pub static SPECIES: [Species; 19] = [
    Species {
        key: "BROADLEAF_S",
        model: 0x9eb7_4001,
        bundle: "HAWAII/DISTRICT_1024094.BNDL",
        label: "small broadleaf tree",
    },
    Species { key: "BROADLEAF_M", model: 0xb4fa_d391, bundle: "HAWAII/DISTRICT_164408.BNDL", label: "broadleaf tree" },
    Species {
        key: "BROADLEAF_L",
        model: 0xe066_3335,
        bundle: "HAWAII/DISTRICT_701575.BNDL",
        label: "large broadleaf tree",
    },
    Species { key: "POPLAR", model: 0xb0d1_73af, bundle: "HAWAII/DISTRICT_701575.BNDL", label: "columnar tree" },
    Species {
        key: "CONIFER_TALL",
        model: 0xa97b_9688,
        bundle: "HAWAII/DISTRICT_701574.BNDL",
        label: "tall conifer with branch cards",
    },
    Species { key: "SPRUCE", model: 0xdc0b_adee, bundle: "HAWAII/DISTRICT_701574.BNDL", label: "dark spruce" },
    Species { key: "SPRUCE_S", model: 0xdeda_583a, bundle: "HAWAII/TRK_UNIT113.BNDL", label: "small spruce" },
    Species {
        key: "AUTUMN",
        model: 0x315f_cb51,
        bundle: "HAWAII/GLOBALRESOURCES.BNDL",
        label: "autumn broadleaf tree",
    },
    Species { key: "AUTUMN_THIN", model: 0x2a4a_616d, bundle: "HAWAII/TRK_UNIT115.BNDL", label: "thin autumn tree" },
    Species { key: "DEAD", model: 0x0c41_aaf0, bundle: "HAWAII/TRK_UNIT146.BNDL", label: "bare tree" },
    Species { key: "BUSH", model: 0xfac8_7374, bundle: "HAWAII/TRK_UNIT1.BNDL", label: "round bush" },
    // The bushes, hedges and grasses of the 2005 world's low growth (`bushes.rs`): small models of the same shader, 2 to 3 m
    // tall. Found by the same census as the trees (a `Tree_*` material, the model under 7 m) and looked at one by one.
    Species { key: "SHRUB_GREEN", model: 0x697e_287c, bundle: "HAWAII/TRK_UNIT33.BNDL", label: "green shrub" },
    Species { key: "HEDGE", model: 0xa807_609b, bundle: "HAWAII/TRK_UNIT158.BNDL", label: "hedge" },
    Species { key: "SHRUB_STRIP", model: 0x5ee2_10c0, bundle: "HAWAII/TRK_UNIT33.BNDL", label: "olive shrub strip" },
    Species { key: "DRY_SHRUB", model: 0x8ca3_1079, bundle: "HAWAII/TRK_UNIT113.BNDL", label: "dry twiggy shrub" },
    Species { key: "DARK_SHRUB", model: 0x3451_85d4, bundle: "HAWAII/TRK_UNIT115.BNDL", label: "dark spiky shrub" },
    Species { key: "PALE_SHRUB", model: 0x7ac6_4a1f, bundle: "HAWAII/TRK_UNIT68.BNDL", label: "pale leaved shrub" },
    Species { key: "GRASS_TALL", model: 0x8f4b_eebd, bundle: "HAWAII/TRK_UNIT158.BNDL", label: "tall grass strip" },
    Species {
        key: "GRASS_CLUMP",
        model: 0xcd87_4a5b,
        bundle: "HAWAII/TRK_UNIT22.BNDL",
        label: "ornamental grass clump",
    },
];

/// The name of the solid of `species` at `level` in the pack's geometry file.
pub fn solid_name(species: &str, level: usize) -> String {
    format!("MW12_{species}_L{level}")
}
