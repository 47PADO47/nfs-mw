//! The speed readout and the nitrous meter, shared by every HUD package that has them: `HUD_SingleRace.fng`,
//! `HUD_Drag.fng` and the split-screen packages (`hud/split`). The objects differ between packages; the rules do not.

use blackbox_feng::{ObjectRef, Runtime};

use super::state::HudState;

/// Language labels of the speed units.
const LABEL_KMH: u32 = 0x8569a25f;
const LABEL_MPH: u32 = 0x8569ab44;
/// The nitrous icon's scripts: while the tank drains, and while it is not draining.
pub(super) const SCRIPT_NOS_BURNING: u32 = 0x77031C70;
pub(super) const SCRIPT_NOS_READY: u32 = 0x03826A28;

/// Starts `script` on `o` unless it is the one playing.
pub(super) fn ensure_script(rt: &mut Runtime, o: ObjectRef, script: u32) {
    if rt.script_of(o) != Some(script) {
        rt.run_script(o, script);
    }
}

const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

/// A decimal digit as text, with no allocation. Anything above 9 shows as 0, as the readout always has.
pub(super) fn digit_text(digit: u32) -> &'static str {
    DIGITS.get(digit as usize).copied().unwrap_or("0")
}

/// The gear digit: `R` in reverse, `N` in neutral, else the gear up to 9.
pub(super) fn gear_text(gear: i32) -> &'static str {
    match gear {
        g if g < 0 => "R",
        0 => "N",
        g => DIGITS[g.min(9) as usize],
    }
}

/// The three speed digits (ones, tens, hundreds) and the unit label. The game shows whole units, and no leading zero
/// (the ones digit always shows).
pub(super) fn speed_readout(rt: &mut Runtime, digits: &[Option<ObjectRef>; 3], units: Option<ObjectRef>, s: &HudState) {
    let speed = s.display_speed();
    let values = [speed % 10, (speed / 10) % 10, (speed / 100) % 10];
    for (i, slot) in digits.iter().enumerate() {
        let Some(o) = *slot else { continue };
        rt.set_text_str(o, digit_text(values[i]));
        let leading_zero = i > 0 && values[i] == 0 && speed < 10u32.pow(i as u32);
        rt.set_hidden(o, leading_zero);
    }
    if let Some(o) = units {
        rt.set_label(o, if s.use_mph { LABEL_MPH } else { LABEL_KMH });
    }
}

/// The objects of a nitrous gauge: the bar (a multi image whose mask turns) and the icon (which plays the burning and
/// ready scripts), and the angle the mask rests at when the tank is empty.
#[derive(Clone, Copy)]
pub(super) struct Nitrous {
    pub bar: Option<ObjectRef>,
    pub icon: Option<ObjectRef>,
    pub empty_deg: f32,
}

/// Turns the bar's mask to the tank level (0..1) and starts the icon's script for the tank's state. `last` is the level
/// the last call had: a falling level is a burning tank.
pub(super) fn nitrous_meter(rt: &mut Runtime, gauge: Nitrous, last: &mut f32, level: f32) {
    let nos = level.clamp(0.0, 1.0);
    if let Some(o) = gauge.bar {
        rt.set_mask_rotation(o, nos * -gauge.empty_deg + gauge.empty_deg);
    }
    if let Some(o) = gauge.icon {
        let script = match nos {
            n if n <= 0.0 => blackbox_feng::runtime::INIT_SCRIPT,
            n if n < *last => SCRIPT_NOS_BURNING,
            _ => SCRIPT_NOS_READY,
        };
        ensure_script(rt, o, script);
    }
    *last = nos;
}

/// The drag HUD's "Perfect Shift!" group (`0x2579C809`) and the script that plays it (one of four of 1900 ms, which one
/// the game picks for which shift is not read), and the share of the red line a shift has to be made above to count
/// **[unconfirmed]**: the game's shift rating is not read.
pub(super) const PERFECT_SHIFT: u32 = 0x2579_C809;
const PERFECT_SCRIPT: u32 = 0xC8D8_8528;
const PERFECT_SHARE: f32 = 0.95;

/// A shift up made close to the red line plays "Perfect Shift!" on `group`. `last` is the gear and rpm of the last call.
pub(super) fn perfect_shift(rt: &mut Runtime, group: Option<ObjectRef>, last: &mut (i32, f32), s: &HudState) {
    let (gear, rpm) = *last;
    *last = (s.gear, s.rpm);
    let Some(o) = group else { return };
    if s.gear > gear && gear >= 1 && rpm >= s.red_line * PERFECT_SHARE {
        rt.run_script(o, PERFECT_SCRIPT);
    }
}
