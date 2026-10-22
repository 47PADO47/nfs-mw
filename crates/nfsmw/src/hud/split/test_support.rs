//! A small stand-in for `HUD_Player1.fng`, for the tests that run without the install: the same names, parents and the
//! scripts the binders start, with the sizes and places of the real package where a test measures them.

use blackbox_feng::package::{
    MessageTargets, MultiDef, ObjectData, ObjectDef, ObjectKind, Package, Script, StringDef, word,
};
use blackbox_feng::{PackageId, Runtime, fe_hash_upper};

use super::names::{self, SCRIPT_REDLINE};

pub const SCRIPT_INIT: u32 = blackbox_feng::runtime::INIT_SCRIPT;
const SCRIPT_SHIFT: u32 = 0x02DD_C8F0;
const SCRIPT_BURNING: u32 = 0x7703_1C70;
const SCRIPT_READY: u32 = 0x0382_6A28;
const LEFT_CONTEXT: u32 = 0x1603_009E;
/// The gauge cluster: the unit text, the gear digit, the shift light and the rpm bar.
const GAUGE_CLUSTER: u32 = 0x5164_D4EA;
/// The first player's gauges answer the widescreen message in a context of their own name.
const RIGHT_CONTEXT_GAUGES: u32 = 0x2FE4_FF9C;

/// The guids of the objects the tests look at.
pub mod guid {
    pub const CLUSTER: u32 = 3;
    pub const GEAR: u32 = 5;
    pub const SHIFT_LIGHT: u32 = 6;
    pub const BAR: u32 = 8;
    pub const DIGIT_1: u32 = 10;
    pub const DIGIT_2: u32 = 11;
    pub const DIGIT_3: u32 = 12;
    pub const NITROUS: u32 = 13;
    pub const NITROUS_BAR: u32 = 15;
    pub const INFO: u32 = 16;
    pub const CLOCK: u32 = 17;
    pub const PLACE: u32 = 19;
    pub const CLOCK_ICON: u32 = 20;
    pub const MESSAGE: u32 = 22;
    pub const COUNTDOWN: u32 = 24;
    pub const WRONG_WAY: u32 = 25;
    pub const LEFT: u32 = 26;
    pub const MAP_BACKING: u32 = 27;
    pub const DIVIDER: u32 = super::names::DIVIDER_GUID;
}

fn object(kind: ObjectKind, guid: u32, name_hash: u32, parent: Option<u32>) -> ObjectDef {
    let mut words = vec![0u32; if kind == ObjectKind::MultiImage { 36 } else { 21 }];
    for (i, v) in [255u32; 4].iter().enumerate() {
        words[i] = *v;
    }
    words[word::ROTATION + 3] = 1.0f32.to_bits();
    let scripts = |ids: &[u32]| ids.iter().map(|id| Script { id: *id, length: 100, ..Default::default() }).collect();
    let mut def = ObjectDef {
        kind,
        guid,
        name_hash,
        flags: 0,
        resource: None,
        parent,
        data: ObjectData { words },
        string: (kind == ObjectKind::String).then(|| StringDef { text: "0".into(), ..Default::default() }),
        multi: (kind == ObjectKind::MultiImage).then(MultiDef::default),
        scripts: match name_hash {
            h if h == fe_hash_upper("Shift_light") => scripts(&[SCRIPT_INIT, SCRIPT_SHIFT]),
            h if h == fe_hash_upper(names::RPM_BAR) => scripts(&[SCRIPT_INIT, SCRIPT_REDLINE]),
            names::NITROUS_ICON => scripts(&[SCRIPT_INIT, SCRIPT_BURNING, SCRIPT_READY]),
            _ => Vec::new(),
        },
        responses: Vec::new(),
    };
    if kind != ObjectKind::Image && kind != ObjectKind::MultiImage {
        def.data.set_f32(word::SIZE, 1.0);
        def.data.set_f32(word::SIZE + 1, 1.0);
    }
    def
}

fn named(kind: ObjectKind, guid: u32, name: &str, parent: Option<u32>) -> ObjectDef {
    object(kind, guid, fe_hash_upper(name), parent)
}

fn sized(mut def: ObjectDef, size: [f32; 2], at: [f32; 2]) -> ObjectDef {
    def.data.set_f32(word::SIZE, size[0]);
    def.data.set_f32(word::SIZE + 1, size[1]);
    def.data.set_f32(word::POSITION, at[0]);
    def.data.set_f32(word::POSITION + 1, at[1]);
    def
}

/// The package: the gauges in the right context (`0x2FE4FF9C`, at 209, -176), the left context with the map backing, the
/// message line, the countdown step, the wrong-way icon and the rule between the halves.
pub fn package(name: &str) -> Package {
    use ObjectKind::{Group, Image, MultiImage, String as Text};
    let objects = vec![
        sized(object(Group, 1, RIGHT_CONTEXT_GAUGES, None), [1.0, 1.0], [209.0, -176.0]),
        object(Group, 2, 0x5C7A_5C8A, Some(1)),
        object(Group, 3, GAUGE_CLUSTER, Some(2)),
        named(Text, 4, "3rdPersonSpeedUnits", Some(3)),
        named(Text, 5, "3rdPersonGear", Some(3)),
        named(Image, 6, "Shift_light", Some(3)),
        object(Group, 7, 0x2A75_9F7A, Some(3)),
        sized(named(Image, 8, names::RPM_BAR, Some(7)), [104.0, 4.0], [0.0, 0.0]),
        named(Group, 9, "SpeedometerGroup", Some(2)),
        named(Text, 10, "SPEED_DIGIT_1", Some(9)),
        named(Text, 11, "SPEED_DIGIT_2", Some(9)),
        named(Text, 12, "SPEED_DIGIT_3", Some(9)),
        object(Group, 13, names::NITROUS_GROUP, Some(2)),
        object(Group, 14, names::NITROUS_ICON, Some(13)),
        object(MultiImage, 15, names::NITROUS_BAR, Some(2)),
        object(Group, 16, names::INFO_GROUP, Some(2)),
        named(Text, 17, names::CLOCK, Some(16)),
        named(Group, 18, "POSITION_GROUP", Some(16)),
        object(Text, 19, 0x9C18_3BF8, Some(18)),
        object(Image, 20, names::CLOCK_ICON, None),
        object(Group, 21, 0x3D2A_ED05, None),
        object(Group, 22, names::GENERIC_MESSAGE, Some(21)),
        object(Text, 23, 0xDA76_8EA6, Some(22)),
        named(Group, 24, names::COUNTDOWN, None),
        object(Image, 25, names::WRONG_WAY, None),
        object(Group, 26, LEFT_CONTEXT, None),
        named(Image, 27, "TRACKMAPTARGETRING", Some(26)),
        sized(object(Image, 28, 0x92D6_630D, None), [1200.0, 4.0], [0.0, 0.0]),
    ];
    let mut objects = objects;
    // The rule's guid is the real package's, so the placement finds it.
    objects[27].guid = names::DIVIDER_GUID;
    Package {
        name: name.into(),
        file_name: String::new(),
        version: 0x20000,
        resources: Vec::new(),
        objects,
        button_count: 0,
        responses: Vec::new(),
        targets: vec![MessageTargets { message: 0x62ED_04EC, guids: vec![1, 26] }],
    }
}

pub fn load(name: &str) -> (Runtime, PackageId) {
    let mut rt = Runtime::new();
    let id = rt.load(package(name));
    (rt, id)
}
