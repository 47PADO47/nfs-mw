//! What the signs and traffic lights conversion takes from the 2012 game and which solids of the 2005 world each piece replaces
//! (`docs/formats/chameleon-signs.md`). The 2012 resources have no names (they are hashes), so every model below was found by
//! looking at renders of the install's models next to the 2005 game's solids; the table is that match, as data. The 2005 side is the
//! name of the solid and the box it spans (measured from the install, metres in the solid's own axes: x across, y along the
//! road for a mast arm or thin for a sign, z up), which a converted model is authored into (`fit = "exact"`).

/// Which way the front of a 2012 sign looks. Its textured plate faces +z or -z of the 2012 engine's axes (x left, y up, z
/// forward); the 2005 signs show their picture towards -y of their own axes, so a model is turned to that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Front {
    PlusZ,
    MinusZ,
}

/// A 2012 model that is read and where it lives.
#[derive(Debug, Clone, Copy)]
pub struct Source {
    /// The model's resource id.
    pub model: u64,
    /// The bundle that holds it, under the install.
    pub bundle: &'static str,
}

/// A road sign of the 2012 game: one mesh (a plate or two on a pole) with one texture, at four levels of detail.
#[derive(Debug, Clone, Copy)]
pub struct Sign {
    pub key: &'static str,
    pub label: &'static str,
    pub source: Source,
    pub front: Front,
}

pub const GLOBAL: &str = "HAWAII/GLOBALRESOURCES.BNDL";

pub static SIGNS: [Sign; 6] = [
    Sign {
        key: "STOP",
        label: "stop sign",
        source: Source { model: 0xaccb_7de9, bundle: "HAWAII/TRK_UNIT6.BNDL" },
        front: Front::PlusZ,
    },
    Sign {
        key: "STOP_OLD",
        label: "weathered stop sign",
        source: Source { model: 0xde9f_ded6, bundle: GLOBAL },
        front: Front::PlusZ,
    },
    Sign {
        key: "SPEED_45",
        label: "speed limit 45",
        source: Source { model: 0xe55b_2ad8, bundle: "HAWAII/TRK_UNIT103.BNDL" },
        front: Front::MinusZ,
    },
    Sign {
        key: "SPEED_55",
        label: "speed limit 55",
        source: Source { model: 0x712a_f22e, bundle: "HAWAII/TRK_UNIT110.BNDL" },
        front: Front::MinusZ,
    },
    Sign {
        key: "NO_ENTRY",
        label: "do not enter",
        source: Source { model: 0xa98f_0117, bundle: "HAWAII/DISTRICT_1024094.BNDL" },
        front: Front::MinusZ,
    },
    Sign {
        key: "TOW_ZONE",
        label: "tow zone and no parking",
        source: Source { model: 0xcf2c_6dd3, bundle: "HAWAII/DISTRICT_1024094.BNDL" },
        front: Front::MinusZ,
    },
];

/// The box of a solid of the 2005 world: its lowest and highest corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box3 {
    pub low: [f32; 3],
    pub high: [f32; 3],
}

const fn b(low: [f32; 3], high: [f32; 3]) -> Box3 {
    Box3 { low, high }
}

/// One solid of the 2005 world that a sign replaces, by name, with its box.
#[derive(Debug, Clone, Copy)]
pub struct SignSwap {
    pub target: &'static str,
    pub sign: &'static str,
    pub old: Box3,
}

/// The signs: every generic regulatory sign of the 2005 world that the 2012 game has a model of. The speed limits are the numbers the
/// 2012 game has (45 and 55; the 2005 game's 35, 50, 60 and 75 keep their own look, a wrong number would be worse than the old
/// picture). The yield, do-not-pass, no-exit and warning signs have no 2012 model (the 2012 warning diamonds take their colour from
/// the instance, which the 2005 scenery does not give).
pub static SIGN_SWAPS: [SignSwap; 8] = [
    SignSwap { target: "XS_STOPSIGNA_1B_00", sign: "STOP", old: b([-0.399, -0.055, -0.010], [0.402, 0.047, 2.230]) },
    SignSwap {
        target: "XS_STOPSIGNB_1B_00",
        sign: "STOP_OLD",
        old: b([-0.555, -0.082, -0.011], [0.497, 0.096, 3.572]),
    },
    SignSwap { target: "XS_SPEED45_1B_00", sign: "SPEED_45", old: b([-0.507, -0.050, -0.294], [0.508, 0.089, 3.244]) },
    SignSwap { target: "XS_SPEED55_1B_00", sign: "SPEED_55", old: b([-0.672, -0.063, -0.389], [0.674, 0.115, 4.313]) },
    SignSwap {
        target: "XS_DONOTENTER_1B_00",
        sign: "NO_ENTRY",
        old: b([-0.698, -0.053, -0.389], [0.709, 0.115, 4.218]),
    },
    SignSwap {
        target: "XS_DONOTENTER_1B_LZ_00",
        sign: "NO_ENTRY",
        old: b([-0.698, -0.053, -0.389], [0.709, 0.115, 4.218]),
    },
    SignSwap {
        target: "XS_NOPARKINGA_1B_00",
        sign: "TOW_ZONE",
        old: b([-0.210, -0.050, -0.027], [0.210, 0.050, 2.234]),
    },
    SignSwap {
        target: "XS_NOPARKANYTIME_1B_00",
        sign: "TOW_ZONE",
        old: b([-0.247, -0.036, -0.010], [0.229, 0.037, 2.352]),
    },
];

/// The traffic light's mast arm of the 2012 game (a pole with an arm over the road and the signal heads hanging from it, in one mesh):
/// the arm is kept and the heads are cut away, because the 2005 world places its heads as separate objects
/// (`XO_TRAFFICLIGHTC` and `D`, hung from the arm). Four variants exist (left and right handed, and the broken ones); one is read.
pub const ARM: Source = Source { model: 0xbb97_a2c5, bundle: GLOBAL };

/// The model that holds one signal head of the 2012 game alone: a pole with a three-lens head. Its second mesh, drawn with the same
/// picture as the arm's heads, is the head: a housing, three lens visors (red on top) and two small visors.
pub const HEAD: Source = Source { model: 0xf613_5e80, bundle: "HAWAII/TRK_UNIT182.BNDL" };

/// The picture the 2012 mast arm and its heads share (the low 24 bits of the texture's resource id).
pub const LIGHT_ATLAS: u64 = 0x09_dbf8;

/// A traffic light solid of the 2005 world that the mast arm replaces: `level` is the level of detail of the 2012 model it takes (0 is
/// the finest).
#[derive(Debug, Clone, Copy)]
pub struct ArmSwap {
    pub target: &'static str,
    pub level: usize,
    pub old: Box3,
}

/// The poles with an arm: `A` has an arm of seven metres, `B` one of 3.5 m, both 8.7 m tall. The `FRAG01` solids are the same pole in
/// the form the game switches to when a pole is knocked over.
pub static ARM_SWAPS: [ArmSwap; 8] = [
    ArmSwap { target: "XO_TRAFFICLIGHTA_1B_00", level: 0, old: b([-0.246, -6.981, -0.044], [0.246, 0.239, 8.671]) },
    ArmSwap { target: "XO_TRAFFICLIGHTA_1C_00", level: 1, old: b([-0.246, -6.981, -0.044], [0.246, 0.239, 8.671]) },
    ArmSwap { target: "XO_TRAFFICLIGHTA_1Z_00", level: 2, old: b([-0.246, -6.981, -0.044], [0.246, 0.239, 8.671]) },
    ArmSwap { target: "TRAFFICLIGHTA_FRAG01", level: 0, old: b([-0.346, -6.981, -0.045], [0.259, 0.442, 8.671]) },
    ArmSwap { target: "XO_TRAFFICLIGHTB_1B_00", level: 0, old: b([-0.246, -3.527, -0.044], [0.246, 0.239, 8.671]) },
    ArmSwap { target: "XO_TRAFFICLIGHTB_1C_00", level: 1, old: b([-0.246, -3.527, -0.044], [0.246, 0.239, 8.671]) },
    ArmSwap { target: "XO_TRAFFICLIGHTB_1Z_00", level: 2, old: b([-0.246, -3.528, -0.044], [0.246, 0.239, 8.671]) },
    ArmSwap { target: "TRAFFICLIGHTB_FRAG01", level: 0, old: b([-0.280, -3.528, -0.101], [0.250, 0.250, 8.671]) },
];

/// How a signal head hangs in the 2005 world: the head's origin is where it hangs from the arm, the head reaches down from it, and its
/// lenses are at the heights the host draws the lit lamp at (`scenes/world/ai/signal_props.rs`, `HEAD_C` and `HEAD_D`): the middle
/// (amber) lens at `centre` below the origin and a lens every `pitch` metres, red on top, green at the bottom.
#[derive(Debug, Clone, Copy)]
pub struct Hanging {
    pub centre: f32,
    pub pitch: f32,
}

/// A signal head of the 2005 world that the 2012 head replaces.
#[derive(Debug, Clone, Copy)]
pub struct HeadSwap {
    pub target: &'static str,
    pub level: usize,
    pub hanging: Hanging,
    pub old: Box3,
}

/// The big head (`C`, 2.5 m tall) and the small one (`D`, 1.8 m); the far level of each is a thin slab.
pub const HANG_C: Hanging = Hanging { centre: -1.15, pitch: 0.42 };
pub const HANG_D: Hanging = Hanging { centre: -0.86, pitch: 0.35 };

pub static HEAD_SWAPS: [HeadSwap; 6] = [
    HeadSwap {
        target: "XO_TRAFFICLIGHTC_1B_00",
        level: 0,
        hanging: HANG_C,
        old: b([-0.243, -0.638, -2.398], [0.539, 0.630, 0.104]),
    },
    HeadSwap {
        target: "XO_TRAFFICLIGHTC_1C_00",
        level: 1,
        hanging: HANG_C,
        old: b([-0.244, -0.640, -2.398], [0.538, 0.629, 0.104]),
    },
    HeadSwap {
        target: "XO_TRAFFICLIGHTC_1Z_00",
        level: 2,
        hanging: HANG_C,
        old: b([-0.010, -0.750, -2.398], [0.129, 0.510, -0.101]),
    },
    HeadSwap {
        target: "XO_TRAFFICLIGHTD_1B_00",
        level: 0,
        hanging: HANG_D,
        old: b([-0.174, -0.444, -1.747], [0.379, 0.450, 0.026]),
    },
    HeadSwap {
        target: "XO_TRAFFICLIGHTD_1C_00",
        level: 1,
        hanging: HANG_D,
        old: b([-0.174, -0.444, -1.747], [0.379, 0.450, 0.026]),
    },
    HeadSwap {
        target: "XO_TRAFFICLIGHTD_1Z_00",
        level: 2,
        hanging: HANG_D,
        old: b([-0.009, -0.438, -1.747], [0.095, 0.450, -0.133]),
    },
];

/// The option kinds and their sets: the value every set adds and what the menu shows for it.
pub const SIGNS_KIND: &str = "road_signs";
pub const SIGNS_KIND_LABEL: &str = "nfsmw2012-road-signs";
pub const LIGHTS_KIND: &str = "traffic_lights";
pub const LIGHTS_KIND_LABEL: &str = "nfsmw2012-traffic-light-models";
pub const VALUE: &str = "mw2012";
pub const VALUE_LABEL: &str = "NFS Most Wanted 2012";
pub const SIGNS_FOLDER: &str = "SIGNS/MW2012";
pub const LIGHTS_FOLDER: &str = "LIGHTS/MW2012";
pub const GEOMETRY_FILE: &str = "GEOMETRY.BIN";
pub const TEXTURES_FILE: &str = "TEXTURES.BIN";
