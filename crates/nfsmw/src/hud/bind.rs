//! Binding: `HudState` in, FEng objects out. The names and rules are the ones the game uses (see
//! `docs/specs/feng-runtime.md` section 8).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::skin::{fill_texture, needle_texture, tach_face_texture};
use super::state::HudState;

/// The script an object plays when its condition is on, and the one it rests in.
const SCRIPT_GREEN: u32 = 0x02DDC8F0;
const SCRIPT_INIT: u32 = blackbox_feng::runtime::INIT_SCRIPT;
/// Language labels of the speed units.
const LABEL_KMH: u32 = 0x8569a25f;
const LABEL_MPH: u32 = 0x8569ab44;
/// The needle sweeps from this angle by this many degrees as the rpm rises.
const NEEDLE_START_DEG: f32 = 66.0;
const NEEDLE_SWEEP_DEG: f32 = 228.0;

/// The objects of the package the HUD drives, found once.
pub struct HudBinding {
    package: PackageId,
    digits: [Option<ObjectRef>; 3],
    units: Option<ObjectRef>,
    gear: Option<ObjectRef>,
    needle: Option<ObjectRef>,
    fill: Option<ObjectRef>,
    face: Option<ObjectRef>,
    shift_light: Option<ObjectRef>,
    shift_on: bool,
    applied_skin: Option<(u8, i32)>,
}

fn find(rt: &Runtime, package: PackageId, name: &str) -> Option<ObjectRef> {
    rt.find(package, fe_hash_upper(name))
}

impl HudBinding {
    /// Finds the objects, and hides everything but the speedometer and the tachometer.
    pub fn new(rt: &mut Runtime, package: PackageId) -> Self {
        let b = Self {
            package,
            digits: [
                find(rt, package, "SPEED_DIGIT_1"),
                find(rt, package, "SPEED_DIGIT_2"),
                find(rt, package, "SPEED_DIGIT_3"),
            ],
            units: find(rt, package, "3rdPersonSpeedUnits"),
            gear: find(rt, package, "3rdPersonGear"),
            needle: find(rt, package, "3rdPersonNeedle"),
            fill: rt.find_guid(package, 0x456e),
            face: find(rt, package, "TAC_Lines_7500"),
            shift_light: find(rt, package, "Shift_light"),
            shift_on: false,
            applied_skin: None,
        };
        b.hide_everything_else(rt);
        b
    }

    /// Only the speedometer group and the gauge cluster are shown for now: the rest of the package (radar,
    /// pursuit bars, race timers) needs game state this build does not have yet.
    fn hide_everything_else(&self, rt: &mut Runtime) {
        let keep: Vec<u32> = ["SpeedometerGroup", "GaugeCluster"]
            .iter()
            .filter_map(|n| find(rt, self.package, n))
            .filter_map(|o| rt.package(self.package).map(|p| p.objects[o.index].guid))
            .collect();
        let Some(def) = rt.package(self.package) else { return };
        let objects = def.objects.clone();
        let by_guid: std::collections::HashMap<u32, usize> =
            objects.iter().enumerate().map(|(i, o)| (o.guid, i)).collect();
        let in_keep = |mut i: usize| {
            // True when the object, or an ancestor, is one to keep.
            loop {
                if keep.contains(&objects[i].guid) {
                    return true;
                }
                match objects[i].parent.and_then(|g| by_guid.get(&g).copied()) {
                    Some(p) if p != i => i = p,
                    _ => return false,
                }
            }
        };
        let is_ancestor_of_keep = |i: usize| {
            keep.iter().filter_map(|g| by_guid.get(g).copied()).any(|mut k| {
                loop {
                    if k == i {
                        return true;
                    }
                    match objects[k].parent.and_then(|g| by_guid.get(&g).copied()) {
                        Some(p) if p != k => k = p,
                        _ => return false,
                    }
                }
            })
        };
        for i in 0..objects.len() {
            if !in_keep(i) && !is_ancestor_of_keep(i) {
                rt.set_hidden(ObjectRef { package: self.package, index: i }, true);
            }
        }
    }

    /// Pushes the state into the objects. Call before `Runtime::update`.
    pub fn apply(&mut self, rt: &mut Runtime, s: &HudState) {
        let speed = s.display_speed();
        let digits = [speed % 10, (speed / 10) % 10, (speed / 100) % 10];
        for (i, slot) in self.digits.iter().enumerate() {
            let Some(o) = *slot else { continue };
            rt.set_text(o, char::from_digit(digits[i], 10).unwrap_or('0').to_string());
            // A leading zero is not shown (the ones digit always is).
            let leading_zero = i > 0 && digits[i] == 0 && speed < 10u32.pow(i as u32);
            rt.set_hidden(o, leading_zero);
        }
        if let Some(o) = self.units {
            rt.set_label(o, if s.use_mph { LABEL_MPH } else { LABEL_KMH });
        }
        if let Some(o) = self.gear {
            let text = match s.gear {
                g if g < 0 => "R".to_string(),
                0 => "N".to_string(),
                g => g.min(9).to_string(),
            };
            rt.set_text(o, text);
        }
        if let Some(o) = self.needle {
            let angle = NEEDLE_START_DEG + s.rpm_fraction() * NEEDLE_SWEEP_DEG;
            rt.set_rotation_z(o, angle.to_radians());
        }
        if let Some(o) = self.shift_light
            && s.shift_light != self.shift_on
        {
            self.shift_on = s.shift_light;
            rt.run_script(o, if s.shift_light { SCRIPT_GREEN } else { SCRIPT_INIT });
        }
        let face_key = (s.skin, (s.max_rpm / 1000.0).ceil() as i32);
        if self.applied_skin != Some(face_key) {
            self.applied_skin = Some(face_key);
            if let Some(o) = self.face {
                rt.set_texture(o, tach_face_texture(s.max_rpm, s.skin));
            }
            if let Some(o) = self.needle {
                rt.set_texture(o, needle_texture(s.skin));
            }
            if let Some(o) = self.fill {
                rt.set_texture(o, fill_texture(s.skin));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use blackbox_feng::package::{ObjectData, ObjectDef, ObjectKind, Package, Script, StringDef};
    use glam::Quat;

    use super::*;

    fn object(kind: ObjectKind, guid: u32, name: &str, parent: Option<u32>) -> ObjectDef {
        let mut words = vec![0u32; 17];
        for (i, v) in [255i32, 255, 255, 255].iter().enumerate() {
            words[i] = *v as u32;
        }
        words[10 + 3] = 1.0f32.to_bits();
        ObjectDef {
            kind,
            guid,
            name_hash: fe_hash_upper(name),
            flags: 0,
            resource: None,
            parent,
            data: ObjectData { words },
            string: (kind == ObjectKind::String).then(|| StringDef { text: "0".into(), ..Default::default() }),
            multi: None,
            scripts: if name == "Shift_light" {
                vec![
                    Script { id: SCRIPT_INIT, ..Default::default() },
                    Script { id: SCRIPT_GREEN, length: 100, ..Default::default() },
                ]
            } else {
                Vec::new()
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
            object(ObjectKind::Group, 11, "RadarGroup", None),
            object(ObjectKind::Image, 12, "RadarBacking", Some(11)),
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

    #[test]
    fn only_the_speedometer_and_the_tachometer_stay() {
        let (rt, id, _) = setup();
        let tree = rt.tree(id);
        let visible = |name: &str| tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper(name)).unwrap().visible;
        assert!(visible("SpeedometerGroup") && visible("GaugeCluster") && visible("3rdPersonNeedle"));
        assert!(!visible("RadarGroup") && !visible("RadarBacking"));
    }

    #[test]
    fn speed_digits_show_the_speed_and_hide_leading_zeros() {
        let (mut rt, id, mut b) = setup();
        b.apply(&mut rt, &HudState { speed: 187.0 / 3.6, ..Default::default() });
        assert_eq!(text_of(&rt, id, "SPEED_DIGIT_1"), ("7".into(), true));
        assert_eq!(text_of(&rt, id, "SPEED_DIGIT_2"), ("8".into(), true));
        assert_eq!(text_of(&rt, id, "SPEED_DIGIT_3"), ("1".into(), true));
        b.apply(&mut rt, &HudState { speed: 7.0 / 3.6, ..Default::default() });
        assert_eq!(text_of(&rt, id, "SPEED_DIGIT_1"), ("7".into(), true));
        assert!(!text_of(&rt, id, "SPEED_DIGIT_2").1 && !text_of(&rt, id, "SPEED_DIGIT_3").1);
        b.apply(&mut rt, &HudState::default());
        assert_eq!(text_of(&rt, id, "SPEED_DIGIT_1"), ("0".into(), true), "the ones digit always shows");
    }

    #[test]
    fn gear_text() {
        let (mut rt, id, mut b) = setup();
        for (g, want) in [(-1, "R"), (0, "N"), (3, "3")] {
            b.apply(&mut rt, &HudState { gear: g, ..Default::default() });
            assert_eq!(text_of(&rt, id, "3rdPersonGear").0, want);
        }
    }

    #[test]
    fn the_needle_sweeps_228_degrees() {
        let (mut rt, id, mut b) = setup();
        let angle = |rt: &Runtime| {
            let tree = rt.tree(id);
            let n = tree.nodes.iter().find(|n| n.name_hash == fe_hash_upper("3rdPersonNeedle")).unwrap();
            (n.local_rotation * Quat::from_rotation_z(0.0)).to_euler(glam::EulerRot::XYZ).2.to_degrees()
        };
        b.apply(&mut rt, &HudState { rpm: 0.0, max_rpm: 8000.0, ..Default::default() });
        assert!((angle(&rt) - 66.0).abs() < 0.1, "{}", angle(&rt));
        b.apply(&mut rt, &HudState { rpm: 4000.0, max_rpm: 8000.0, ..Default::default() });
        // 66 + 114 = 180: the quaternion folds it to -180 or 180.
        assert!((angle(&rt).abs() - 180.0).abs() < 0.1, "{}", angle(&rt));
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
}
