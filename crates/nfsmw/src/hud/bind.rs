//! Binding: `HudState` in, FEng objects out. The names, scripts and rules are the ones the game uses (see
//! `docs/specs/feng-runtime.md` section 8).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::blip_style::BlipStyle;
use super::blips::BlipBinding;
use super::elements::{Layout, Shown};
use super::minimap::MinimapBinding;
use super::pursuit::timer_text;
use super::skin::{drag_face_texture, fill_texture, needle_texture, tach_face_texture};
use super::state::{HudState, redline_rotation, redline_rotation_drag};

/// The script an object plays when its condition is on, and the one it rests in.
const SCRIPT_GREEN: u32 = 0x02DDC8F0;
pub(super) const SCRIPT_INIT: u32 = blackbox_feng::runtime::INIT_SCRIPT;
/// The rear-view mirror's top group (`0x7fac287b`) and the script that brings it to full colour.
const MIRROR_FRAME: u32 = 0x7FAC_287B;
/// The radar's outer fade group. The detector binding owns the layouts under it.
const RADAR_BACKING_GROUP: u32 = 0xAB9C_1D4D;
/// The mirror fades in over this many seconds, the length of the game's own mirror track (`len 600`).
const MIRROR_FADE_SECONDS: f32 = 0.6;

/// The mirror's alpha `elapsed` seconds into its fade-in: 0 at the start, 255 once the fade is done.
pub fn mirror_fade_alpha(elapsed: f32) -> u8 {
    (elapsed / MIRROR_FADE_SECONDS).clamp(0.0, 1.0).mul_add(255.0, 0.5) as u8
}
/// The needle sweeps from this angle by this many degrees as the rpm rises.
const NEEDLE_START_DEG: f32 = 66.0;
const NEEDLE_SWEEP_DEG: f32 = 228.0;
/// The nitrous bar's mask turns from this angle (empty) down to 0 (full).
const NOS_BAR_EMPTY_DEG: f32 = 175.0;
/// The turbo needle swings across this range of angles for a gauge from `TURBO_MIN_PSI` to `TURBO_MAX_PSI`.
const TURBO_DEG: (f32, f32) = (-45.0, 45.0);
const TURBO_PSI: (f32, f32) = (-20.0, 20.0);
/// The drag needle (`Drag_RPM_Needle` is named `3rdPersonNeedle` as well) rests at its authored 42 degrees and runs over
/// the `Drag_RPM_10000_Lines` face; the sweep is the main needle's **[unconfirmed]**.
const DRAG_NEEDLE_START_DEG: f32 = 42.0;
const DRAG_SCALE_RPM: f32 = 10000.0;
/// The gear digit's alpha while a gear change is in progress.
const GEAR_SHIFTING_ALPHA: u8 = 0x88;
/// The pursuit board's group (`PursuitBoardGroup`).
const PURSUIT_BOARD_GROUP: u32 = 0xDE89_E070;
/// The cops group (`0xE0B1430B`, the parent of the cops counters): its icons and the taken-out count beside them.
/// Its fade-in is a script on a message this build does not raise, so it is held at full alpha during a chase too.
const COPS_GROUP: u32 = 0xE0B1_430B;
/// The gear digit inside the tachometer group (`3rdPersonGear`). The Original theme's gear slot emits it on its own.
pub(super) const GEAR: &str = "3rdPersonGear";

/// The objects of the package the HUD drives, found once.
pub struct HudBinding {
    gauge_colors: super::gauge_colors::Binding,
    police: super::police::Binding,
    race: super::race::Binding,
    leaderboard: super::leaderboard::Binding,
    pursuit_meter: super::pursuit_meter::Binding,
    hiding_mark: super::hiding_mark::HidingMark,
    layout: Layout,
    /// The rear-view mirror is shown; the detector chooses its corresponding authored layout.
    mirror_shown: bool,
    /// The mirror's top group and the group under it, held at full alpha (see `apply`).
    mirror_frames: [Option<ObjectRef>; 2],
    /// Seconds since the HUD started, up to the end of the mirror's fade-in.
    mirror_elapsed: f32,
    digits: [Option<ObjectRef>; 3],
    units: Option<ObjectRef>,
    gear: Option<ObjectRef>,
    needle: Option<ObjectRef>,
    /// The drag HUD's "Perfect Shift!" group and the gear and rpm the last frame had (`perfect_shift`).
    perfect: Option<ObjectRef>,
    last_shift: (i32, f32),
    /// The package is `HUD_Drag.fng`: its needle rests at 42 degrees and runs over a 10000 rpm face.
    drag: bool,
    fill: Option<ObjectRef>,
    face: Option<ObjectRef>,
    redline: Option<ObjectRef>,
    shift_light: Option<ObjectRef>,
    nos_icon: Option<ObjectRef>,
    nos_bar: Option<ObjectRef>,
    speedbreaker: super::speedbreaker::Binding,
    turbo_needle: Option<ObjectRef>,
    turbo_faces: Vec<ObjectRef>,
    /// The pursuit board's timer, `mm:ss.cc`.
    pursuit_timer: Option<ObjectRef>,
    /// The cops taken out in the chase (`Cops_TakenOut`, beside `Cops_TakenOut_Icon`): the wrecked count.
    cops_taken: Option<ObjectRef>,
    /// The pursuit board and cops groups, held at full alpha during a chase (see `pursuit`).
    pursuit_frames: Vec<ObjectRef>,
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

impl HudBinding {
    /// Finds the objects, and hides everything but the speedometer and the tachometer.
    pub fn new(rt: &mut Runtime, package: PackageId) -> Self {
        let mut b = Self {
            gauge_colors: super::gauge_colors::Binding::new(rt, package),
            police: super::police::Binding::new(rt, package),
            race: super::race::Binding::new(rt, package),
            leaderboard: super::leaderboard::Binding::new(rt, package),
            pursuit_meter: super::pursuit_meter::Binding::new(rt, package),
            hiding_mark: super::hiding_mark::HidingMark::new(rt, package),
            layout: Layout::new(rt, package),
            mirror_frames: [None; 2],
            mirror_shown: true,
            mirror_elapsed: 0.0,
            digits: [
                find(rt, package, "SPEED_DIGIT_1"),
                find(rt, package, "SPEED_DIGIT_2"),
                find(rt, package, "SPEED_DIGIT_3"),
            ],
            units: find(rt, package, "3rdPersonSpeedUnits"),
            gear: find(rt, package, GEAR),
            needle: find(rt, package, "3rdPersonNeedle"),
            perfect: rt.find(package, super::gauges::PERFECT_SHIFT),
            last_shift: (0, 0.0),
            drag: rt.package(package).is_some_and(|p| p.name.eq_ignore_ascii_case("HUD_Drag.fng")),
            fill: rt.find_guid(package, 0x456e),
            face: find(rt, package, "TAC_Lines_7500"),
            redline: find(rt, package, "RPM_REDLINE"),
            shift_light: find(rt, package, "Shift_light"),
            nos_icon: rt.find(package, 0x27DD_F583),
            nos_bar: rt.find(package, 0xEDFB_6D37),
            speedbreaker: super::speedbreaker::Binding::new(rt, package),
            turbo_needle: find(rt, package, "3rdperson_TurboDial"),
            turbo_faces: rt.tree(package).nodes.iter().filter_map(|n| {
                matches!(n.kind, blackbox_feng::NodeKind::Image { texture, .. } if texture == fe_hash_upper("TURBO_LINES"))
                    .then_some(ObjectRef { package, index: n.index })
            }).collect(),
            pursuit_timer: find(rt, package, "PursuitTime"),
            cops_taken: find(rt, package, "Cops_TakenOut"),
            pursuit_frames: [PURSUIT_BOARD_GROUP, COPS_GROUP].iter().filter_map(|h| rt.find(package, *h)).collect(),
            minimap: None,
            blips: BlipBinding::builtin(rt, package),
            shift_on: false,
            last_nos: 0.0,
            applied_face: None,
        };
        b.layout.update(rt, Shown { mirror: true, speedbreaker: true, ..Default::default() });
        // The mirror's frame rests fully visible. The game fades it in from a script on an event this build does
        // not raise, so the fade is skipped here.
        b.mirror_frames = [MIRROR_FRAME, RADAR_BACKING_GROUP].map(|h| rt.find(package, h));
        b
    }

    /// Gives the HUD a minimap (built from the package's objects and the map's calibration). Without one the
    /// minimap objects stay hidden.
    /// Registers a map blip style (see [`BlipStyle`]); its objects come from the package by its icon names. Call it
    /// before the viewport is built (`HudViewport::new` takes the icons from `blip_icon_hashes`).
    pub fn add_blip_style(&mut self, rt: &mut Runtime, package: PackageId, style: Box<dyn BlipStyle>) {
        self.blips.add(rt, package, style);
    }

    /// The name hashes of the map blips' pool objects, for `HudViewport::new`.
    pub fn blip_icon_hashes(&self) -> Vec<u32> {
        self.blips.icon_hashes()
    }

    pub fn with_minimap(mut self, minimap: Option<MinimapBinding>) -> Self {
        self.minimap = minimap;
        self
    }

    /// The minimap, to switch its map variant.
    pub fn minimap_mut(&mut self) -> Option<&mut MinimapBinding> {
        self.minimap.as_mut()
    }

    /// Pushes the state into the objects. Call before `Runtime::update`.
    /// Shows or hides the rear-view mirror. The detector switches layouts at the next `apply`.
    pub fn set_mirror_shown(&mut self, on: bool) {
        self.mirror_shown = on;
    }

    /// Advances the mirror's fade-in by `dt` seconds. Called once per frame, before `apply`.
    pub fn advance_mirror_fade(&mut self, dt: f32) {
        self.police.advance(dt);
        self.race.advance(dt);
        self.leaderboard.advance(dt);
        self.mirror_elapsed = (self.mirror_elapsed + dt).min(MIRROR_FADE_SECONDS);
    }

    pub fn advance_blips(&mut self, dt: f32) {
        self.blips.advance(dt);
    }

    pub fn apply(&mut self, rt: &mut Runtime, s: &HudState) {
        let map = s.minimap.zip(self.minimap.as_ref());
        let shown = Shown {
            nitrous: s.has_nos,
            turbo: s.has_turbo,
            // The race HUD always carries the speed breaker meter; its charge only moves the bar and the icon.
            speedbreaker: true,
            minimap: map.is_some(),
            laser: s.cop_nearby,
            // The detector's own scripts hide its two layouts and preserve their exit animations.
            radar: true,
            heat: s.vehicle_heat >= 1.0,
            mirror: self.mirror_shown,
            pursuit: s.pursuit.is_some(),
            cooldown: s.pursuit.is_some_and(|p| p.cooldown_shown()),
            career: s.career_bounty.is_some(),
            race: s.race.is_some(),
            countdown: s.race.is_some_and(|r| r.count.is_some()),
            leaders: s.leaders.as_ref().map_or(0, |l| l.shown_rows() as u8),
            message: s.message.is_some() || self.race.message_active(),
        };
        self.layout.update(rt, shown);
        // The mirror's frame rests at full alpha, every frame: its own colour track would otherwise leave it at zero.
        let alpha = mirror_fade_alpha(self.mirror_elapsed);
        for frame in self.mirror_frames.iter().flatten() {
            if !s.speedbreaker_active {
                rt.set_alpha(*frame, alpha);
            }
        }
        if let Some((at, minimap)) = map {
            minimap.apply(rt, s, &at);
        }
        if let Some(minimap) = self.minimap.as_ref() {
            self.blips.apply(rt, s, minimap);
        }
        self.speedometer(rt, s);
        self.tachometer(rt, s);
        self.speedbreaker.update(rt, s.speedbreaker, s.speedbreaker_active);
        self.pursuit(rt, s);
        self.race.apply(rt, s.race);
        self.leaderboard.apply(rt, s);
        self.race.show_message(rt, s.message.as_ref());
        self.pursuit_meter.apply(rt, s.pursuit);
        self.hiding_mark.apply(rt, s.pursuit.is_some_and(|p| p.hiding_shown()));
        self.police.apply(rt, s, self.mirror_shown);
        if s.has_nos {
            self.nitrous(rt, s);
        }
        if s.has_turbo {
            self.turbo(rt, s);
        }
    }

    /// Dynamic widths keep their fixed edges after any authored script changes in this frame.
    pub fn finish_frame(&mut self, rt: &mut Runtime, s: &HudState) {
        self.pursuit_meter.finish_frame(rt, s.pursuit);
        self.gauge_colors.apply(rt, s.gauge_colors);
    }

    fn speedometer(&self, rt: &mut Runtime, s: &HudState) {
        super::gauges::speed_readout(rt, &self.digits, self.units, s);
    }

    fn tachometer(&mut self, rt: &mut Runtime, s: &HudState) {
        if let Some(o) = self.gear {
            rt.set_text_str(o, super::gauges::gear_text(s.gear));
            let alpha = if s.shifting { GEAR_SHIFTING_ALPHA } else { 255 };
            rt.set_colour_rgba(o, [0, 0, 0, alpha]);
        }
        if let Some(o) = self.needle {
            let angle = match self.drag {
                true => DRAG_NEEDLE_START_DEG + (s.rpm / DRAG_SCALE_RPM).clamp(0.0, 1.0) * NEEDLE_SWEEP_DEG,
                false => NEEDLE_START_DEG + s.rpm_fraction() * NEEDLE_SWEEP_DEG,
            };
            rt.set_rotation_z(o, angle.to_radians());
        }
        self.perfect_shift(rt, s);
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
            let face = match self.drag {
                true => drag_face_texture(s.max_rpm),
                false => tach_face_texture(s.max_rpm, s.skin),
            };
            rt.set_texture(o, face);
        }
        // The skins are the race HUD's; the drag gauge keeps its own needle.
        if !self.drag {
            for &o in &self.turbo_faces {
                rt.set_texture(o, fe_hash_upper(&format!("TURBO_LINES_{:02}", s.skin)));
            }
            if let Some(o) = self.turbo_needle {
                rt.set_texture(o, fe_hash_upper(&format!("TURBO_NEEDLE_{:02}", s.skin)));
            }
            if let Some(o) = self.needle {
                rt.set_texture(o, needle_texture(s.skin));
            }
            if let Some(o) = self.fill {
                rt.set_texture(o, fill_texture(s.skin));
            }
        }
        if let Some(o) = self.redline {
            let angle = match self.drag {
                true => redline_rotation_drag(s.max_rpm, s.red_line),
                false => redline_rotation(s.max_rpm, s.red_line),
            };
            rt.set_mask_rotation(o, angle);
        }
    }

    /// A drag shift made close to the red line plays "Perfect Shift!".
    fn perfect_shift(&mut self, rt: &mut Runtime, s: &HudState) {
        let group = self.perfect.filter(|_| self.drag);
        super::gauges::perfect_shift(rt, group, &mut self.last_shift, s);
    }

    /// The pursuit board's timer and the width of the cool-down bar, while a chase runs. The board and the bar fade
    /// in from scripts the game starts on a message this build does not raise, so their groups are held at full
    /// alpha here instead (as the mirror is, see `apply`).
    fn pursuit(&self, rt: &mut Runtime, s: &HudState) {
        let Some(p) = s.pursuit else { return };
        for frame in &self.pursuit_frames {
            rt.set_alpha(*frame, 255);
        }
        if let Some(o) = self.pursuit_timer {
            rt.set_text(o, timer_text(p.secs));
        }
        if let Some(o) = self.cops_taken {
            rt.set_text(o, p.cops_destroyed.to_string());
        }
    }

    fn nitrous(&mut self, rt: &mut Runtime, s: &HudState) {
        super::gauges::nitrous_meter(
            rt,
            super::gauges::Nitrous { bar: self.nos_bar, icon: self.nos_icon, empty_deg: NOS_BAR_EMPTY_DEG },
            &mut self.last_nos,
            s.nos,
        );
    }

    fn turbo(&self, rt: &mut Runtime, s: &HudState) {
        let Some(o) = self.turbo_needle else { return };
        let boost = (s.boost_psi - TURBO_PSI.0) / (TURBO_PSI.1 - TURBO_PSI.0);
        let angle = -(TURBO_DEG.0 + boost * (TURBO_DEG.1 - TURBO_DEG.0));
        rt.set_rotation_z(o, angle.to_radians());
    }
}
