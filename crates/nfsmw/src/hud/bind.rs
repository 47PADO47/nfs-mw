//! Binding: `HudState` in, FEng objects out. The names, scripts and rules are the ones the game uses (see
//! `docs/specs/feng-runtime.md` section 8).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};
use glam::Vec3;

use super::blip_style::BlipStyle;
use super::blips::BlipBinding;
use super::elements::Layout;
use super::minimap::MinimapBinding;
use super::skin::{fill_texture, needle_texture, tach_face_texture};
use super::state::{HudState, redline_rotation};

/// The script an object plays when its condition is on, and the one it rests in.
const SCRIPT_GREEN: u32 = 0x02DDC8F0;
pub(super) const SCRIPT_INIT: u32 = blackbox_feng::runtime::INIT_SCRIPT;
/// The nitrous icon's scripts: while the tank drains, and while it is not draining.
pub(super) const SCRIPT_NOS_BURNING: u32 = 0x77031C70;
pub(super) const SCRIPT_NOS_READY: u32 = 0x03826A28;
/// The rear-view mirror's top group (`0x7fac287b`) and the script that brings it to full colour.
const MIRROR_FRAME: u32 = 0x7FAC_287B;
/// The mirror's group of bezel images under it (`0x9ee06631`).
const MIRROR_GROUP: u32 = 0x9EE0_6631;
/// The radar's backing group (`0xab9c1d4d`) and its icon and LED group (`0x062743f5`).
const RADAR_BACKING_GROUP: u32 = 0xAB9C_1D4D;
const RADAR_ICON_GROUP: u32 = 0x0627_43F5;
/// The radar's backing image (`0x839e7d77`). The package puts it over the picture; the reference screenshot has it
/// just under the bezel, so both radar pieces move down by these amounts (docs/specs/rear-view-mirror.md).
const RADAR_BACKING: u32 = 0x839E_7D77;
/// The mirror fades in over this many seconds, the length of the game's own mirror track (`len 600`).
const MIRROR_FADE_SECONDS: f32 = 0.6;

/// The mirror's alpha `elapsed` seconds into its fade-in: 0 at the start, 255 once the fade is done.
pub fn mirror_fade_alpha(elapsed: f32) -> u8 {
    (elapsed / MIRROR_FADE_SECONDS).clamp(0.0, 1.0).mul_add(255.0, 0.5) as u8
}
const RADAR_BACKING_DROP: f32 = 75.0;
const RADAR_GROUP_DROP: f32 = 15.0;
/// Language labels of the speed units.
const LABEL_KMH: u32 = 0x8569a25f;
const LABEL_MPH: u32 = 0x8569ab44;
/// The needle sweeps from this angle by this many degrees as the rpm rises.
const NEEDLE_START_DEG: f32 = 66.0;
const NEEDLE_SWEEP_DEG: f32 = 228.0;
/// The nitrous bar's mask turns from this angle (empty) down to 0 (full).
const NOS_BAR_EMPTY_DEG: f32 = 175.0;
/// The speed breaker bar: the FEng object `Speedbreaker_Meter_Multi_Image` (name hash of that string). Its geometry is
/// assumed to match the nitrous bar's (not checked on screen yet).
const SPEEDBREAKER_BAR: u32 = 0x5DF2_F5B4;
const SPEEDBREAKER_BAR_EMPTY_DEG: f32 = NOS_BAR_EMPTY_DEG;
/// The turbo needle swings across this range of angles for a gauge from `TURBO_MIN_PSI` to `TURBO_MAX_PSI`.
const TURBO_DEG: (f32, f32) = (-45.0, 45.0);
const TURBO_PSI: (f32, f32) = (-20.0, 20.0);
/// The gear digit's alpha while a gear change is in progress.
const GEAR_SHIFTING_ALPHA: u8 = 0x88;

/// The objects of the package the HUD drives, found once.
pub struct HudBinding {
    layout: Layout,
    /// The rear-view mirror and the radar under it are shown (the `rear_view_mirror` setting).
    mirror_shown: bool,
    /// The mirror's top group and the group under it, held at full alpha (see `apply`).
    mirror_frames: [Option<ObjectRef>; 4],
    /// Seconds since the HUD started, up to the end of the mirror's fade-in.
    mirror_elapsed: f32,
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
    speedbreaker_bar: Option<ObjectRef>,
    turbo_needle: Option<ObjectRef>,
    /// The minimap, once the host has given it a map (see [`HudBinding::with_minimap`]).
    minimap: Option<MinimapBinding>,
    /// The cop, racer, checkpoint and helicopter objects on the map.
    blips: BlipBinding,
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
            mirror_frames: [None; 4],
            mirror_shown: true,
            mirror_elapsed: 0.0,
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
            speedbreaker_bar: rt.find(package, SPEEDBREAKER_BAR),
            turbo_needle: find(rt, package, "3rdperson_TurboDial"),
            minimap: None,
            blips: BlipBinding::builtin(rt, package),
            shift_on: false,
            last_nos: 0.0,
            applied_face: None,
        };
        b.layout.update(rt, false, false, false, false, true);
        // The mirror's frame rests fully visible. The game fades it in from a script on an event this build does
        // not raise, so the fade is skipped here.
        b.mirror_frames =
            [MIRROR_FRAME, MIRROR_GROUP, RADAR_BACKING_GROUP, RADAR_ICON_GROUP].map(|h| rt.find(package, h));
        for (hash, drop) in [(RADAR_BACKING, RADAR_BACKING_DROP), (RADAR_ICON_GROUP, RADAR_GROUP_DROP)] {
            if let Some(o) = rt.find(package, hash)
                && let Some(p) = rt.position(o)
            {
                rt.set_position(o, p + Vec3::Y * drop);
            }
        }
        b
    }

    /// Gives the HUD a minimap (built from the package's objects and the map's calibration). Without one the
    /// minimap objects stay hidden.
    /// Registers a custom map blip style (see [`BlipStyle`]); its objects come from the package by its icon names.
    /// No caller yet: the extension point for custom blips.
    #[allow(dead_code)]
    pub fn add_blip_style(&mut self, rt: &mut Runtime, package: PackageId, style: Box<dyn BlipStyle>) {
        self.blips.add(rt, package, style);
    }

    pub fn with_minimap(mut self, minimap: Option<MinimapBinding>) -> Self {
        self.minimap = minimap;
        self
    }

    /// Pushes the state into the objects. Call before `Runtime::update`.
    /// Shows or hides the rear-view mirror and the radar under it. Takes effect at the next `apply`.
    pub fn set_mirror_shown(&mut self, on: bool) {
        self.mirror_shown = on;
    }

    /// Advances the mirror's fade-in by `dt` seconds. Called once per frame, before `apply`.
    pub fn advance_mirror_fade(&mut self, dt: f32) {
        self.mirror_elapsed = (self.mirror_elapsed + dt).min(MIRROR_FADE_SECONDS);
    }

    pub fn apply(&mut self, rt: &mut Runtime, s: &HudState) {
        let map = s.minimap.zip(self.minimap.as_ref());
        self.layout.update(rt, s.has_nos, s.has_turbo, map.is_some(), s.cop_nearby, self.mirror_shown);
        // The mirror's frame rests at full alpha, every frame: its own colour track would otherwise leave it at zero.
        let alpha = mirror_fade_alpha(self.mirror_elapsed);
        for frame in self.mirror_frames.iter().flatten() {
            rt.set_alpha(*frame, alpha);
        }
        if let Some((at, minimap)) = map {
            minimap.apply(rt, s, &at);
        }
        if let Some(minimap) = self.minimap.as_ref() {
            self.blips.apply(rt, s, minimap);
        }
        self.speedometer(rt, s);
        self.tachometer(rt, s);
        self.speedbreaker(rt, s);
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

    fn speedbreaker(&mut self, rt: &mut Runtime, s: &HudState) {
        if let Some(o) = self.speedbreaker_bar {
            let meter = s.speedbreaker.clamp(0.0, 1.0);
            rt.set_mask_rotation(o, meter * -SPEEDBREAKER_BAR_EMPTY_DEG + SPEEDBREAKER_BAR_EMPTY_DEG);
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
