//! The `shiftpattern` class: the engine's RPM shape through a gear change and the shift sounds.
//! Rules: `docs/specs/engine-sound-effects.md` §1.

use super::fields::{Fields, bezier_points, shift_pair};

/// One RPM move of an up shift: `rpm` and `time_ms` scale a unit Bezier curve whose four control points
/// `(x, y)` are in `curve` (`x` is the fraction of the time, `y` the fraction of the RPM).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShiftStage {
    pub rpm: i32,
    pub time_ms: i32,
    pub curve: [[f32; 2]; 4],
}

impl ShiftStage {
    /// The point `t` (0 to 1) of the curve as `(milliseconds, rpm offset)`.
    pub fn point(&self, t: f32) -> (f32, f32) {
        let u = 1.0 - t;
        let w = [u * u * u, 3.0 * t * u * u, 3.0 * t * t * u, t * t * t];
        let mut x = 0.0;
        let mut y = 0.0;
        for (weight, p) in w.iter().zip(&self.curve) {
            x += weight * p[0];
            y += weight * p[1];
        }
        (x * self.time_ms as f32, y * self.rpm as f32)
    }
}

/// One `shiftpattern` collection.
#[derive(Debug, Clone, PartialEq)]
pub struct ShiftSound {
    /// The collection's name or hash.
    pub name: String,
    /// The gear sound bank in `SOUND/SHIFTING` (`GEAR_MED_Lev3.abk`).
    pub bank: String,
    /// Seconds after the shift starts that the gear clunk plays.
    pub up_sound_delay: f32,
    pub down_sound_delay: f32,
    /// Clunk volumes, 0 to 32767.
    pub up_volume: u32,
    pub down_volume: u32,
    /// The extra engine volume right after an up shift engages (a fraction) and how long it takes to fade.
    pub up_engage_attack_volume: f32,
    pub up_engage_attack_ms: u32,
    /// Up shift: the drop while the clutch is out (one or two stages) and the engage.
    pub up_disengage_fall: Vec<ShiftStage>,
    pub up_engage: Option<ShiftStage>,
    /// Down shift: `(rpm, ms)` of the four stages (disengage fall, engage rise, engage fall) and the reattach
    /// scale (ms per RPM of difference).
    pub down_disengage_fall: (u32, u32),
    pub down_engage_rise: (u32, u32),
    pub down_engage_fall: (u32, u32),
    pub down_reattach_scale: f32,
    /// The wobble after an up shift: amplitude (RPM, volume), period and decay time (ms).
    pub lfo_rpm: Wobble,
    pub lfo_volume: Wobble,
}

/// A decaying sine: `amplitude * sin(2 pi t / period)` falling to 0 over `decay_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Wobble {
    pub amplitude: u32,
    pub period_ms: u32,
    pub decay_ms: u32,
}

impl ShiftSound {
    pub(super) fn read(c: Fields<'_>) -> Self {
        let curves: Vec<[[f32; 2]; 4]> =
            c.raw_items("Up_DisengageFall_Curve").into_iter().filter_map(bezier_points).collect();
        let stage = |bytes: &[u8], curve: Option<&[[f32; 2]; 4]>| {
            let (rpm, time_ms) = shift_pair(bytes)?;
            Some(ShiftStage { rpm, time_ms, curve: *curve? })
        };
        let up_disengage_fall = c
            .raw_items("Up_DisengageFall")
            .into_iter()
            .enumerate()
            .filter_map(|(i, bytes)| stage(bytes, curves.get(i)))
            .collect();
        let engage_curve = c.raw_items("Up_Engage_Curve").first().copied().and_then(bezier_points);
        let up_engage = c.raw_items("Up_Engage").first().and_then(|bytes| stage(bytes, engage_curve.as_ref()));
        let pair = |rpm: &str, time: &str| (c.u32(rpm), c.u32(time));
        Self {
            name: c.name(),
            bank: c.string("BankName"),
            up_sound_delay: c.f32("Up_Shift_Sound_Delay"),
            down_sound_delay: c.f32("Down_Shift_Sound_Delay"),
            up_volume: c.u32("Up_Vol_Shift"),
            down_volume: c.u32("Down_Vol_Shift"),
            up_engage_attack_volume: c.f32("Up_Engaging_Attack_Vol"),
            up_engage_attack_ms: c.u32("Up_Engaging_Attack_T"),
            up_disengage_fall,
            up_engage,
            down_disengage_fall: pair("Down_Disengage_Fall_RPM", "Down_Disengage_Fall_T"),
            down_engage_rise: pair("Down_Engaging_Rise_RPM", "Down_Engaging_Rise_T"),
            down_engage_fall: pair("Down_Engaging_Fall_RPM", "Down_Engaging_Fall_T"),
            down_reattach_scale: c.f32("Down_Reattach_Scale"),
            lfo_rpm: Wobble {
                amplitude: c.u32("LFO_RPM_Amp"),
                period_ms: c.u32("LFO_RPM_Freq"),
                decay_ms: c.u32("LFO_RPM_Decay_Time"),
            },
            lfo_volume: Wobble {
                amplitude: c.u32("LFO_Vol_Amp"),
                period_ms: c.u32("LFO_Vol_Freq"),
                decay_ms: c.u32("LFO_Vol_Decay_Time"),
            },
        }
    }
}
