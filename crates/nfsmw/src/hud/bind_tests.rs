use blackbox_feng::package::{MultiDef, ObjectData, ObjectDef, ObjectKind, Package, Script, StringDef};
use blackbox_feng::{NodeKind, PackageId, Runtime, fe_hash_upper};
use glam::Quat;

use super::bind::{HudBinding, SCRIPT_INIT};
use super::gauges::{SCRIPT_NOS_BURNING, SCRIPT_NOS_READY};
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
            0x1144_8B5C => scripts(&[SCRIPT_INIT, SCRIPT_NOS_BURNING]),
            0x839E_7D77 | 0x9EE0_6631 => super::pursuit_tests::visibility_scripts(),
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
        // The rear-view mirror: its top group, the bezel group under it, the bezel, and the radar's backing.
        named(ObjectKind::Group, 20, 0x7FAC_287B, None),
        named(ObjectKind::Group, 21, 0x9EE0_6631, Some(20)),
        named(ObjectKind::Image, 22, 0x5630_E0B9, Some(21)),
        named(ObjectKind::Image, 23, 0x839E_7D77, None),
        object(ObjectKind::Group, 24, super::speedbreaker::GROUP, None),
        object(ObjectKind::Group, 25, super::speedbreaker::ICON, Some(24)),
        object(ObjectKind::MultiImage, 26, super::speedbreaker::MASK, Some(24)),
        named(ObjectKind::Group, 27, super::speedbreaker::LINEAR_GROUP, None),
        named(ObjectKind::Image, 28, super::speedbreaker::LINEAR_BAR, Some(27)),
        // The heat meter group (flame and multiplier) that rings the minimap.
        named(ObjectKind::Group, 29, 0xC46A_80A9, None),
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

#[test]
fn speedbreaker_gauge_and_icon_show_and_follow_original_charge_scripts() {
    let mut package = package();
    for (guid, ids) in
        [(25, vec![SCRIPT_INIT, 0x7703_1C70]), (27, vec![0x5B0D_9106]), (28, vec![0x026D_ED57, 0x620E_4851])]
    {
        package.objects.iter_mut().find(|o| o.guid == guid).unwrap().scripts =
            ids.into_iter().map(|id| Script { id, length: 100, ..Default::default() }).collect();
    }
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let bar = rt.find(id, super::speedbreaker::LINEAR_BAR).unwrap();
    rt.set_size_xy(bar, 240.0, 12.0);
    rt.set_position_xy(bar, 139.5, 10.0);
    let mut binding = HudBinding::new(&mut rt, id);
    let group = rt.find(id, super::speedbreaker::LINEAR_GROUP).unwrap();
    rt.run_script(group, 0x5B0D_9106);
    let icon = rt.find(id, fe_hash_upper(super::speedbreaker::ICON)).unwrap();
    for charge in [0.0, 0.1, 0.3, 0.300001, 0.7, 1.0] {
        binding.apply(&mut rt, &HudState { speedbreaker: charge, ..Default::default() });
        for guid in [24, 25, 26] {
            assert!(visible(&rt, id, guid), "Speedbreaker object {guid} is visible");
        }
        assert!((mask_rotation(&rt, id, 26) - 175.0 * (1.0 - charge)).abs() < 1e-4);
        assert_eq!(
            rt.script_of(icon),
            Some(match charge > 0.0 {
                true => 0x7703_1C70,
                false => SCRIPT_INIT,
            })
        );
        assert_eq!(
            rt.script_of(bar),
            Some(match charge <= 0.3 {
                true => 0x026D_ED57,
                false => 0x620E_4851,
            })
        );
        let size = rt.size(bar).unwrap();
        let at = rt.position(bar).unwrap();
        assert!((size.x - 240.0 * charge).abs() < 1e-4);
        assert!((at.x - size.x / 2.0 - 19.5).abs() < 1e-4, "preserved left edge");
    }
}

#[test]
fn the_heat_meter_goes_with_the_minimap() {
    // This binding has no minimap: with heat the ring would show around a hole in the HUD.
    let (mut rt, id, mut b) = setup();
    b.apply(&mut rt, &HudState { vehicle_heat: 3.0, ..Default::default() });
    assert!(!visible(&rt, id, 29), "no minimap, no heat ring");
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
fn the_speedbreaker_meter_shows_for_the_whole_race_whatever_its_charge() {
    let (mut rt, id, mut b) = setup();
    for meter in [0.0, 0.5] {
        b.apply(&mut rt, &HudState { speedbreaker: meter, ..Default::default() });
        for guid in [24, 25, 26] {
            assert!(visible(&rt, id, guid), "{guid} at meter {meter}");
        }
    }
}

#[test]
fn the_speedbreaker_bar_empties_as_the_meter_drains() {
    let (mut rt, id, mut b) = setup();
    for (meter, want) in [(1.0, 0.0), (0.5, 87.5), (0.0, 175.0)] {
        b.apply(&mut rt, &HudState { speedbreaker: meter, ..Default::default() });
        assert!((mask_rotation(&rt, id, 26) - want).abs() < 1e-3, "{meter}");
    }
}

#[test]
fn the_speedbreaker_icon_is_lit_while_the_meter_holds_charge() {
    let (mut rt, id, mut b) = setup();
    let icon = rt.find(id, 0x1144_8B5C).unwrap();
    b.apply(&mut rt, &HudState { speedbreaker: 0.4, ..Default::default() });
    assert_eq!(rt.script_of(icon), Some(SCRIPT_NOS_BURNING), "charged");
    b.apply(&mut rt, &HudState { speedbreaker: 0.0, ..Default::default() });
    assert_eq!(rt.script_of(icon), Some(SCRIPT_INIT), "empty");
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

#[test]
fn the_mirror_fades_in_over_the_game_track_length() {
    use super::bind::mirror_fade_alpha;
    assert_eq!(mirror_fade_alpha(0.0), 0);
    assert_eq!(mirror_fade_alpha(0.3), 128);
    assert_eq!(mirror_fade_alpha(0.6), 255);
    assert_eq!(mirror_fade_alpha(5.0), 255, "it stays at full once the fade is done");
}

#[test]
fn the_mirror_setting_leaves_the_standalone_police_radar_available() {
    let (mut rt, id, mut b) = setup();
    // Past the fade, so the frame's alpha is full and only the setting can hide the mirror.
    b.advance_mirror_fade(1.0);
    b.apply(&mut rt, &HudState::default());
    rt.update(1.0);
    let mirror = [20, 21, 22];
    assert!(!visible(&rt, id, 23), "mirror mode hides the separate standalone backing");
    for guid in mirror {
        assert!(visible(&rt, id, guid), "{guid} shows by default");
    }
    b.set_mirror_shown(false);
    b.apply(&mut rt, &HudState::default());
    rt.update(1.0);
    for guid in mirror {
        assert!(!visible(&rt, id, guid), "{guid} hides with rear_view_mirror off");
    }
    assert!(visible(&rt, id, 23), "standalone radar is independent of the mirror setting");
    assert!(visible(&rt, id, 1) && visible(&rt, id, 8), "the speedometer and tachometer stay");
    b.set_mirror_shown(true);
    b.apply(&mut rt, &HudState::default());
    rt.update(1.0);
    assert!(!visible(&rt, id, 23), "the standalone backing hides again");
    for guid in mirror {
        assert!(visible(&rt, id, guid), "{guid} shows again");
    }
}

#[test]
fn the_mirror_frame_fades_in_then_holds_full_alpha() {
    let (mut rt, id, mut b) = setup();
    let frame = rt.find(id, 0x7FAC_287B).unwrap();
    b.apply(&mut rt, &HudState::default());
    assert_eq!(rt.alpha(frame), Some(0), "the fade starts at zero");
    b.advance_mirror_fade(0.3);
    b.apply(&mut rt, &HudState::default());
    assert_eq!(rt.alpha(frame), Some(128));
    b.advance_mirror_fade(1.0);
    b.apply(&mut rt, &HudState::default());
    assert_eq!(rt.alpha(frame), Some(255), "full once the fade is done");
}

#[test]
fn gauge_colors_survive_script_updates_preserve_alpha_and_restore_authored_rgb() {
    let (mut rt, id, mut binding) = setup();
    let needle = rt.find(id, fe_hash_upper("3rdPersonNeedle")).unwrap();
    let original = rt.tree(id).nodes.iter().find(|n| n.index == needle.index).unwrap().colour;
    let mut state = HudState {
        gauge_colors: [Some([20, 80, 160]), Some([100, 220, 30]), Some([80, 180, 250])],
        ..Default::default()
    };
    binding.apply(&mut rt, &state);
    rt.update(0.1);
    rt.set_alpha(needle, 123);
    binding.finish_frame(&mut rt, &state);
    let tree = rt.tree(id);
    assert_eq!(tree.nodes.iter().find(|n| n.index == needle.index).unwrap().colour, [100, 220, 30, 123]);
    let digits = rt.find(id, fe_hash_upper("SPEED_DIGIT_1")).unwrap();
    assert_eq!(&tree.nodes.iter().find(|n| n.index == digits.index).unwrap().colour[..3], &[80, 180, 250]);
    state.gauge_colors = [None; 3];
    binding.apply(&mut rt, &state);
    rt.update(0.1);
    binding.finish_frame(&mut rt, &state);
    let restored = rt.tree(id).nodes.iter().find(|n| n.index == needle.index).unwrap().colour;
    assert_eq!(&restored[..3], &original[..3]);
}

#[test]
fn launch_needle_complements_current_gauge_color_and_restores_it_without_flicker() {
    let (mut rt, id, mut binding) = setup();
    let needle = rt.find(id, fe_hash_upper("3rdPersonNeedle")).unwrap();
    let original = rt.colour_rgba(needle).unwrap();
    let mut state =
        HudState { perfect_launch_range: true, gauge_colors: [None, Some([100, 220, 30]), None], ..Default::default() };
    for _ in 0..3 {
        binding.apply(&mut rt, &state);
        rt.update(1.0 / 144.0);
        rt.set_alpha(needle, 123);
        binding.finish_frame(&mut rt, &state);
        assert_eq!(rt.colour_rgba(needle), Some([155, 35, 225, 255]));
    }
    state.gauge_colors[1] = Some([10, 20, 30]);
    binding.finish_frame(&mut rt, &state);
    assert_eq!(rt.colour_rgba(needle), Some([245, 235, 225, 255]));
    state.perfect_launch_range = false;
    binding.finish_frame(&mut rt, &state);
    assert_eq!(rt.colour_rgba(needle), Some([10, 20, 30, 255]));
    state.perfect_launch_range = true;
    binding.finish_frame(&mut rt, &state);
    state.gauge_colors[1] = None;
    binding.finish_frame(&mut rt, &state);
    assert_eq!(rt.colour_rgba(needle), Some([!original[0], !original[1], !original[2], 255]));
    state.perfect_launch_range = false;
    binding.finish_frame(&mut rt, &state);
    assert_eq!(rt.colour_rgba(needle), Some([original[0], original[1], original[2], 255]));
}
