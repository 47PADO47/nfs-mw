//! The split-screen tachometer: no dial, a bar. The rpm bar's width follows the rpm with its left edge fixed, and its
//! redline script plays at the red line. The gear digit and the shift light are the dial HUD's (`Tachometer::Update`
//! and `DragTachometer::Update` in the decomp, their split-screen branches) **[decomp]**.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::Kind;
use super::names::{RPM_BAR, SCRIPT_REDLINE};
use crate::hud::gauges::ensure_script;
use crate::hud::state::HudState;

/// The script the shift light plays when the gearbox wants the next gear.
const SCRIPT_SHIFT: u32 = 0x02DD_C8F0;
/// The drag shift light's green pulse (`PulseGreen`) **[unconfirmed]**: the drag package pulses blue, green or red by how
/// good the shift is; the state has only "shift now".
const SCRIPT_DRAG_SHIFT: u32 = 0x2A15_F4B9;
/// The gear digit's alpha while a gear change is in progress.
const GEAR_SHIFTING_ALPHA: u8 = 0x88;

/// The rpm bar as authored: where its left edge and its middle are, and how wide it is when the engine is at its limit.
struct Bar {
    object: ObjectRef,
    left: f32,
    y: f32,
    width: f32,
    height: f32,
}

pub struct Tach {
    kind: Kind,
    gear: Option<ObjectRef>,
    bar: Option<Bar>,
    shift_light: Option<ObjectRef>,
    shift_on: bool,
}

impl Tach {
    pub fn new(rt: &Runtime, package: PackageId, kind: Kind) -> Self {
        let find = |name: &str| rt.find(package, fe_hash_upper(name));
        let bar = find(RPM_BAR).and_then(|object| {
            let size = rt.size(object)?;
            let at = rt.position(object)?;
            Some(Bar { object, left: at.x - size.x / 2.0, y: at.y, width: size.x, height: size.y })
        });
        Self { kind, gear: find("3rdPersonGear"), bar, shift_light: find("Shift_light"), shift_on: false }
    }

    pub fn apply(&mut self, rt: &mut Runtime, s: &HudState) {
        self.gear(rt, s);
        self.bar(rt, s);
        self.shift_light(rt, s);
    }

    fn gear(&self, rt: &mut Runtime, s: &HudState) {
        let Some(o) = self.gear else { return };
        rt.set_text(o, gear_letter(s.gear));
        // The race HUD leaves the digit's colour as authored; the drag one fades it white while the gear changes.
        if self.kind == Kind::Drag {
            let alpha = if s.shifting { GEAR_SHIFTING_ALPHA } else { 255 };
            rt.set_colour_rgba(o, [255, 255, 255, alpha]);
        }
    }

    fn bar(&self, rt: &mut Runtime, s: &HudState) {
        let Some(bar) = &self.bar else { return };
        let fraction = if s.max_rpm > 0.0 { (s.rpm / s.max_rpm).clamp(0.0, 1.0) } else { 0.0 };
        let width = bar.width * fraction;
        rt.set_size_xy(bar.object, width, bar.height);
        rt.set_position_xy(bar.object, bar.left + width / 2.0, bar.y);
        let script = match s.rpm >= s.red_line {
            true => SCRIPT_REDLINE,
            false => blackbox_feng::runtime::INIT_SCRIPT,
        };
        ensure_script(rt, bar.object, script);
    }

    fn shift_light(&mut self, rt: &mut Runtime, s: &HudState) {
        let Some(o) = self.shift_light else { return };
        // The drag light is a big ring that rests lit (its `INIT` script chains to a pulse), so it is hidden between shifts.
        if self.kind == Kind::Drag {
            rt.set_hidden(o, !s.shift_light);
        }
        if s.shift_light == self.shift_on {
            return;
        }
        self.shift_on = s.shift_light;
        let on = match self.kind {
            Kind::Race => SCRIPT_SHIFT,
            Kind::Drag => SCRIPT_DRAG_SHIFT,
        };
        rt.run_script(o, if s.shift_light { on } else { blackbox_feng::runtime::INIT_SCRIPT });
    }
}

/// The gear digit: `R`, `N`, or the gear.
pub(super) fn gear_letter(gear: i32) -> String {
    match gear {
        g if g < 0 => "R".to_string(),
        0 => "N".to_string(),
        g => g.min(9).to_string(),
    }
}
