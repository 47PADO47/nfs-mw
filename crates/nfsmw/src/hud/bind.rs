//! Binding: `HudState` in, FEng objects out. The names, scripts and rules are the ones the game uses (see
//! `docs/specs/feng-runtime.md` section 8).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::elements::Layout;
use super::skin::{fill_texture, needle_texture, tach_face_texture};
use super::state::{HudState, redline_rotation};

/// The script an object plays when its condition is on, and the one it rests in.
const SCRIPT_GREEN: u32 = 0x02DDC8F0;
pub(super) const SCRIPT_INIT: u32 = blackbox_feng::runtime::INIT_SCRIPT;
/// The nitrous icon's scripts: while the tank drains, and while it is not draining.
pub(super) const SCRIPT_NOS_BURNING: u32 = 0x77031C70;
pub(super) const SCRIPT_NOS_READY: u32 = 0x03826A28;
/// Language labels of the speed units.
const LABEL_KMH: u32 = 0x8569a25f;
const LABEL_MPH: u32 = 0x8569ab44;
/// The needle sweeps from this angle by this many degrees as the rpm rises.
const NEEDLE_START_DEG: f32 = 66.0;
const NEEDLE_SWEEP_DEG: f32 = 228.0;
/// The nitrous bar's mask turns from this angle (empty) down to 0 (full).
const NOS_BAR_EMPTY_DEG: f32 = 175.0;
/// The turbo needle swings across this range of angles for a gauge from `TURBO_MIN_PSI` to `TURBO_MAX_PSI`.
const TURBO_DEG: (f32, f32) = (-45.0, 45.0);
const TURBO_PSI: (f32, f32) = (-20.0, 20.0);
/// The gear digit's alpha while a gear change is in progress.
const GEAR_SHIFTING_ALPHA: u8 = 0x88;

/// The objects of the package the HUD drives, found once.
pub struct HudBinding {
    layout: Layout,
    digits: [Option<ObjectRef>; 3],
    units: Option<ObjectRef>,
    gear: Option<ObjectRef>,
    needle: Option<ObjectRef>,
    fill: Option<ObjectRef>,
    face: Option<ObjectRef>,
    redline: Option<ObjectRef>,
    shift_light: Option<ObjectRef>,
    nos_icon: Option<ObjectRef>,
    nos_bar: Option<ObjectRef>,
    turbo_needle: Option<ObjectRef>,
    shift_on: bool,
    last_nos: f32,
    /// What the face, needle and redline were last set for: skin, scale, red line.
    applied_face: Option<(u8, u32, u32)>,
}

fn find(rt: &Runtime, package: PackageId, name: &str) -> Option<ObjectRef> {
    rt.find(package, fe_hash_upper(name))
}

/// Starts `script` on `o` unless it is the one playing.
fn ensure_script(rt: &mut Runtime, o: ObjectRef, script: u32) {
    if rt.script_of(o) != Some(script) {
        rt.run_script(o, script);
    }
}

impl HudBinding {
    /// Finds the objects, and hides everything but the speedometer and the tachometer.
    pub fn new(rt: &mut Runtime, package: PackageId) -> Self {
        let mut b = Self {
            layout: Layout::new(rt, package),
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
            redline: find(rt, package, "RPM_REDLINE"),
            shift_light: find(rt, package, "Shift_light"),
            nos_icon: rt.find(package, 0x27DD_F583),
            nos_bar: rt.find(package, 0xEDFB_6D37),
            turbo_needle: find(rt, package, "3rdperson_TurboDial"),
            shift_on: false,
            last_nos: 0.0,
            applied_face: None,
        };
        b.layout.update(rt, false, false);
        b
    }

    /// Pushes the state into the objects. Call before `Runtime::update`.
    pub fn apply(&mut self, rt: &mut Runtime, s: &HudState) {
        self.layout.update(rt, s.has_nos, s.has_turbo);
        self.speedometer(rt, s);
        self.tachometer(rt, s);
        if s.has_nos {
            self.nitrous(rt, s);
        }
        if s.has_turbo {
            self.turbo(rt, s);
        }
    }

    fn speedometer(&self, rt: &mut Runtime, s: &HudState) {
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
    }

    fn tachometer(&mut self, rt: &mut Runtime, s: &HudState) {
        if let Some(o) = self.gear {
            let text = match s.gear {
                g if g < 0 => "R".to_string(),
                0 => "N".to_string(),
                g => g.min(9).to_string(),
            };
            rt.set_text(o, text);
            let alpha = if s.shifting { GEAR_SHIFTING_ALPHA } else { 255 };
            rt.set_colour_rgba(o, [0, 0, 0, alpha]);
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
        let face_key = (s.skin, s.scale_rpm() as u32, s.red_line as u32);
        if self.applied_face == Some(face_key) {
            return;
        }
        self.applied_face = Some(face_key);
        if let Some(o) = self.face {
            rt.set_texture(o, tach_face_texture(s.max_rpm, s.skin));
        }
        if let Some(o) = self.needle {
            rt.set_texture(o, needle_texture(s.skin));
        }
        if let Some(o) = self.fill {
            rt.set_texture(o, fill_texture(s.skin));
        }
        if let Some(o) = self.redline {
            rt.set_mask_rotation(o, redline_rotation(s.max_rpm, s.red_line));
        }
    }

    fn nitrous(&mut self, rt: &mut Runtime, s: &HudState) {
        let nos = s.nos.clamp(0.0, 1.0);
        if let Some(o) = self.nos_bar {
            rt.set_mask_rotation(o, nos * -NOS_BAR_EMPTY_DEG + NOS_BAR_EMPTY_DEG);
        }
        if let Some(o) = self.nos_icon {
            let script = match nos {
                n if n <= 0.0 => SCRIPT_INIT,
                n if n < self.last_nos => SCRIPT_NOS_BURNING,
                _ => SCRIPT_NOS_READY,
            };
            ensure_script(rt, o, script);
        }
        self.last_nos = nos;
    }

    fn turbo(&self, rt: &mut Runtime, s: &HudState) {
        let Some(o) = self.turbo_needle else { return };
        let boost = (s.boost_psi - TURBO_PSI.0) / (TURBO_PSI.1 - TURBO_PSI.0);
        let angle = -(TURBO_DEG.0 + boost * (TURBO_DEG.1 - TURBO_DEG.0));
        rt.set_rotation_z(o, angle.to_radians());
    }
}
