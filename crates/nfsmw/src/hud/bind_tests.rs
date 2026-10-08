use blackbox_feng::package::{MultiDef, ObjectData, ObjectDef, ObjectKind, Package, Script, StringDef};
use blackbox_feng::{NodeKind, PackageId, Runtime, fe_hash_upper};
use glam::Quat;

use super::bind::{HudBinding, SCRIPT_INIT, SCRIPT_NOS_BURNING, SCRIPT_NOS_READY};
use super::state::HudState;

const SCRIPT_GREEN: u32 = 0x02DDC8F0;
const LABEL_KMH: u32 = 0x8569a25f;
const LABEL_MPH: u32 = 0x8569ab44;

fn object(kind: ObjectKind, guid: u32, name: &str, parent: Option<u32>) -> ObjectDef {
    named(kind, guid, fe_hash_upper(name), parent)
}

fn named(kind: ObjectKind, guid: u32, name_hash: u32, parent: Option<u32>) -> ObjectDef {
    let mut words = vec![0u32; if kind == ObjectKind::MultiImage { 36 } else { 17 }];
    for (i, v) in [255i32, 255, 255, 255].iter().enumerate() {
        words[i] = *v as u32;
    }
    words[10 + 3] = 1.0f32.to_bits();
    let scripts = |ids: &[u32]| ids.iter().map(|id| Script { id: *id, length: 100, ..Default::default() }).collect();
    ObjectDef {
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
            h if h == fe_hash_upper("Shift_light") => scripts(&[SCRIPT_INIT, SCRIPT_GREEN]),
            0x27DD_F583 => scripts(&[SCRIPT_INIT, SCRIPT_NOS_BURNING, SCRIPT_NOS_READY]),
            _ => Vec::new(),
        },
        responses: Vec::new(),
    }
}

fn package() -> Package {
    let objects = vec![
        object(ObjectKind::Group, 1, "SpeedometerGroup", None),
        object(ObjectKind::String, 2, "SPEED_DIGIT_1", Some(1)),
        object(ObjectKind::String, 3, "SPEED_DIGIT_2", Some(1)),
        object(ObjectKind::String, 4, "SPEED_DIGIT_3", Some(1)),
        object(ObjectKind::Group, 5, "GaugeCluster", None),
        object(ObjectKind::String, 6, "3rdPersonSpeedUnits", Some(5)),
        object(ObjectKind::String, 7, "3rdPersonGear", Some(5)),
        object(ObjectKind::Image, 8, "3rdPersonNeedle", Some(5)),
        object(ObjectKind::Image, 9, "Shift_light", Some(5)),
        object(ObjectKind::Image, 10, "TAC_Lines_7500", Some(5)),
        object(ObjectKind::MultiImage, 11, "RPM_REDLINE", Some(5)),
        object(ObjectKind::Group, 12, "RadarGroup", None),
        object(ObjectKind::Image, 13, "RadarBacking", Some(12)),
        // Nitrous: a group with the icon, and the bar next to it, both under a cluster group.
        object(ObjectKind::Group, 14, "Cluster", None),
        named(ObjectKind::Group, 15, 0x87c3_8e97, Some(14)),
        named(ObjectKind::Group, 16, 0x27DD_F583, Some(15)),
        named(ObjectKind::MultiImage, 17, 0xEDFB_6D37, Some(14)),
        object(ObjectKind::Group, 18, "TURBO_GROUP", Some(14)),
        object(ObjectKind::Image, 19, "3rdperson_TurboDial", Some(18)),
    ];
    Package {
        name: "HUD.fng".into(),
        file_name: String::new(),
        version: 0x20000,
        resources: Vec::new(),
        objects,
        button_count: 0,
        responses: Vec::new(),
        targets: Vec::new(),
    }
}

fn setup() -> (Runtime, PackageId, HudBinding) {
    let mut rt = Runtime::new();
    let id = rt.load(package());
    let binding = HudBinding::new(&mut rt, id);
    (rt, id, binding)
}

fn text_of(rt: &Runtime, id: PackageId, name: &str) -> (String, bool) {
    let tree = rt.tree(id);
    let n = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
    (n.text.clone().unwrap_or_default(), n.visible)
}

fn visible(rt: &Runtime, id: PackageId, guid: u32) -> bool {
    rt.tree(id).nodes.iter().find(|n| n.guid == guid).unwrap().visible
}

fn angle_of(rt: &Runtime, id: PackageId, name: &str) -> f32 {
    let tree = rt.tree(id);
    let n = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap();
    (n.local_rotation * Quat::from_rotation_z(0.0)).to_euler(glam::EulerRot::XYZ).2.to_degrees()
}

fn mask_rotation(rt: &Runtime, id: PackageId, guid: u32) -> f32 {
    let tree = rt.tree(id);
    let n = tree.nodes.iter().find(|n| n.guid == guid).unwrap();
    let NodeKind::Image { mask_rotation, .. } = n.kind else { panic!("not an image") };
    mask_rotation[2]
}

#[test]
fn only_the_speedometer_and_the_tachometer_stay_without_gauges() {
    let (rt, id, _) = setup();
    for guid in [1, 5, 8] {
        assert!(visible(&rt, id, guid), "{guid}");
    }
    for guid in [12, 13, 15, 16, 17, 18, 19] {
        assert!(!visible(&rt, id, guid), "{guid} must be hidden");
    }
}

#[test]
fn the_nitrous_and_turbo_gauges_show_with_the_car_that_has_them() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &HudState { has_nos: true, ..Default::default() });
    for guid in [14, 15, 16, 17] {
        assert!(visible(&rt, id, guid), "{guid}");
    }
    assert!(!visible(&rt, id, 18) && !visible(&rt, id, 19), "no turbo");
    b.apply(&mut rt, &HudState { has_nos: true, has_turbo: true, ..Default::default() });
    assert!(visible(&rt, id, 19) && visible(&rt, id, 17));
    b.apply(&mut rt, &HudState::default());
    assert!(!visible(&rt, id, 15) && !visible(&rt, id, 19), "gone with the hardware");
    assert!(visible(&rt, id, 1) && visible(&rt, id, 8), "the speedometer and tachometer stay");
}

#[test]
fn speed_digits_show_the_speed_and_hide_leading_zeros() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &HudState { speed: 187.5 / 3.6, ..Default::default() });
    assert_eq!(text_of(&rt, id, "SPEED_DIGIT_1"), ("7".into(), true));
    assert_eq!(text_of(&rt, id, "SPEED_DIGIT_2"), ("8".into(), true));
    assert_eq!(text_of(&rt, id, "SPEED_DIGIT_3"), ("1".into(), true));
    b.apply(&mut rt, &HudState { speed: 7.5 / 3.6, ..Default::default() });
    assert_eq!(text_of(&rt, id, "SPEED_DIGIT_1"), ("7".into(), true));
    assert!(!text_of(&rt, id, "SPEED_DIGIT_2").1 && !text_of(&rt, id, "SPEED_DIGIT_3").1);
    b.apply(&mut rt, &HudState::default());
    assert_eq!(text_of(&rt, id, "SPEED_DIGIT_1"), ("0".into(), true), "the ones digit always shows");
}

#[test]
fn gear_text_and_its_dimming_while_shifting() {
    let (mut rt, id, mut b) = setup();
    for (g, want) in [(-1, "R"), (0, "N"), (3, "3")] {
        b.apply(&mut rt, &HudState { gear: g, ..Default::default() });
        assert_eq!(text_of(&rt, id, "3rdPersonGear").0, want);
    }
    let alpha = |rt: &Runtime| {
        let tree = rt.tree(id);
        tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper("3rdPersonGear")).unwrap().colour[3]
    };
    b.apply(&mut rt, &HudState { gear: 2, shifting: true, ..Default::default() });
    assert_eq!(alpha(&rt), 0x88);
    b.apply(&mut rt, &HudState { gear: 3, ..Default::default() });
    assert_eq!(alpha(&rt), 255);
}

#[test]
fn the_needle_sweeps_228_degrees_over_the_face_not_over_the_redline() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &HudState { rpm: 0.0, max_rpm: 8250.0, ..Default::default() });
    assert!((angle_of(&rt, id, "3rdPersonNeedle") - 66.0).abs() < 0.1);
    // MAX_RPM 8250 has the 9000 face: half of it is 4500 rpm, 66 + 114 = 180 degrees (folded to -180 or 180).
    b.apply(&mut rt, &HudState { rpm: 4500.0, max_rpm: 8250.0, red_line: 8000.0, ..Default::default() });
    assert!((angle_of(&rt, id, "3rdPersonNeedle").abs() - 180.0).abs() < 0.1);
}

#[test]
fn the_redline_mask_turns_to_the_table_value() {
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &HudState { max_rpm: 8250.0, red_line: 8000.0, ..Default::default() });
    assert_eq!(mask_rotation(&rt, id, 11), 154.0);
    b.apply(&mut rt, &HudState { max_rpm: 8250.0, red_line: 7000.0, ..Default::default() });
    assert_eq!(mask_rotation(&rt, id, 11), 127.0);
}

#[test]
fn the_nitrous_bar_empties_as_the_tank_does() {
    let (mut rt, id, mut b) = setup();
    for (nos, want) in [(1.0, 0.0), (0.5, 87.5), (0.0, 175.0)] {
        b.apply(&mut rt, &HudState { has_nos: true, nos, ..Default::default() });
        assert!((mask_rotation(&rt, id, 17) - want).abs() < 1e-3, "{nos}");
    }
}

#[test]
fn the_nitrous_icon_switches_scripts_with_the_tank() {
    let (mut rt, id, mut b) = setup();
    let icon = rt.find(id, 0x27DD_F583).unwrap();
    b.apply(&mut rt, &HudState { has_nos: true, nos: 1.0, ..Default::default() });
    assert_eq!(rt.script_of(icon), Some(SCRIPT_NOS_READY), "a full tank");
    b.apply(&mut rt, &HudState { has_nos: true, nos: 0.9, ..Default::default() });
    assert_eq!(rt.script_of(icon), Some(SCRIPT_NOS_BURNING), "draining");
    b.apply(&mut rt, &HudState { has_nos: true, nos: 0.9, ..Default::default() });
    assert_eq!(rt.script_of(icon), Some(SCRIPT_NOS_READY), "no longer draining");
    b.apply(&mut rt, &HudState { has_nos: true, nos: 0.0, ..Default::default() });
    assert_eq!(rt.script_of(icon), Some(SCRIPT_INIT), "empty");
}

#[test]
fn the_turbo_needle_follows_the_boost() {
    let (mut rt, id, mut b) = setup();
    for (psi, want) in [(-20.0, 45.0), (0.0, 0.0), (20.0, -45.0), (10.0, -22.5)] {
        b.apply(&mut rt, &HudState { has_turbo: true, boost_psi: psi, ..Default::default() });
        assert!((angle_of(&rt, id, "3rdperson_TurboDial") - want).abs() < 0.1, "{psi} psi");
    }
}

#[test]
fn the_shift_light_switches_scripts_only_on_change() {
    let (mut rt, id, mut b) = setup();
    let shift = rt.find(id, fe_hash_upper("Shift_light")).unwrap();
    rt.update(0.02);
    assert_eq!(rt.script_of(shift), Some(SCRIPT_INIT));
    b.apply(&mut rt, &HudState { shift_light: true, ..Default::default() });
    assert_eq!(rt.script_of(shift), Some(SCRIPT_GREEN));
    rt.update(0.05);
    b.apply(&mut rt, &HudState { shift_light: true, ..Default::default() });
    assert!(rt.object(shift).unwrap().time > 0, "the script keeps running");
    b.apply(&mut rt, &HudState { shift_light: false, ..Default::default() });
    assert_eq!(rt.script_of(shift), Some(SCRIPT_INIT));
}

#[test]
fn the_units_follow_the_setting() {
    let (mut rt, id, mut b) = setup();
    rt.set_string_resolver(|l| match l {
        LABEL_KMH => Some("KM/H".into()),
        LABEL_MPH => Some("MPH".into()),
        _ => None,
    });
    b.apply(&mut rt, &HudState::default());
    assert_eq!(text_of(&rt, id, "3rdPersonSpeedUnits").0, "KM/H");
    b.apply(&mut rt, &HudState { use_mph: true, ..Default::default() });
    assert_eq!(text_of(&rt, id, "3rdPersonSpeedUnits").0, "MPH");
}
