//! Lamp marker roles. Player appearance uses the installed profiles in `car_light_flares`.
use blackbox_hash::bstring_hash;
use blackbox_render::EffectVertex;
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;

use super::vehicle_effects::geometry;
mod damage;
#[cfg(test)]
mod mod_car_tests;
use damage::marker_flags;

const HEAD_MARKERS: [&str; 3] = ["LEFT_HEADLIGHT", "CENTRE_HEADLIGHT", "RIGHT_HEADLIGHT"];
const TAIL_MARKERS: [&str; 2] = ["LEFT_BRAKELIGHT", "RIGHT_BRAKELIGHT"];
const CENTRE_MARKERS: [&str; 2] = ["BRAKELIGHT_CENTRE", "CENTRE_BRAKELIGHT"];
/// `CENTRE_REVERSE` is an extension for a mod car whose reverse lamp sits on the centre plane (it has no damage bit).
const REVERSE_MARKERS: [&str; 3] = ["LEFT_REVERSE", "RIGHT_REVERSE", "CENTRE_REVERSE"];
const SIGNAL_MARKERS: [(&str, bool); 4] =
    [("LEFT_HEADLIGHT", true), ("RIGHT_HEADLIGHT", false), ("LEFT_BRAKELIGHT", true), ("RIGHT_BRAKELIGHT", false)];
/// The cop light bar: red lamps and blue lamps.
const SIREN_RED: [&str; 1] = ["COPLIGHTRED"];
const SIREN_BLUE: [&str; 1] = ["COPLIGHTBLUE"];
/// Two markers closer than this (metres) are the same lamp, seen in two solids.
const SAME_LAMP: f32 = 0.01;
// Existing optional traffic signals; the player flare path does not use them.
const SIGNAL_RADIUS: f32 = 0.3;
const SIGNAL_BRIGHTNESS: f32 = 0.9;
const SIGNAL_COLOUR: [f32; 4] = [1.0, 0.55, 0.05, 1.0];
const SIGNAL_STEER: f32 = 0.3;
const SIGNAL_PERIOD: f32 = 1.0 / 1.5;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Head,
    Tail,
    Reverse,
    CentreBrake,
    Signal(bool),
    /// A cop light bar lamp: red (true) or blue.
    Siren(bool),
    SirenBright(bool),
    SirenWhite,
    SirenOrange,
}

/// One lamp: its place in the car model's frame, metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lamp {
    pub at: Vec3,
    pub direction: Vec3,
    pub kind: Kind,
    /// Original VehicleFX identity; zero for markers without a mapped light flag.
    pub flags: u32,
}

/// What lights the lamps of a car at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LampState {
    /// Original independent broken-light bits, applied after normal lamp admission.
    pub broken: u32,
    /// Enabled original VehicleFX emergency-light bits; waveform uses `time`.
    pub emergency: u32,
    /// The brake pedal, 0 to 1.
    pub brake: f32,
    pub reverse: bool,
    /// Driver steering, retained in the feed; it does not control racer lamps.
    pub steer: f32,
    /// The headlights are on.
    pub headlights: bool,
    /// Optional enhanced optics; original flare profiles remain unchanged when false.
    pub enhanced: bool,
    /// Turn signals are enabled (the `turn_signals` setting).
    pub turn_signals: bool,
    /// Simulated seconds, for the blinking.
    pub time: f32,
}

impl LampState {
    /// The cached shared brake surfaces cannot remain ON when every original brake bit is broken.
    pub fn brake_surfaces_on(self) -> bool {
        self.brake > 0.0 && self.broken & 56 != 56
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CarLights {
    lamps: Vec<Lamp>,
}

impl CarLights {
    /// The lamps of every placed solid of `model`.
    pub fn for_model(model: &CarModel) -> Self {
        let heads = HEAD_MARKERS.map(bstring_hash);
        let tails = TAIL_MARKERS.map(bstring_hash);
        let reverses = REVERSE_MARKERS.map(bstring_hash);
        let signals = SIGNAL_MARKERS.map(|(name, left)| (bstring_hash(name), left));
        let centres = CENTRE_MARKERS.map(bstring_hash);
        let reds = SIREN_RED.map(bstring_hash);
        let blues = SIREN_BLUE.map(bstring_hash);
        let mut lamps: Vec<Lamp> = Vec::new();
        for placement in &model.placements {
            let Some(solid) = model.solids.get(&placement.solid) else { continue };
            for marker in &solid.markers {
                let hash = marker.name_hash;
                let mut kinds = Vec::new();
                if heads.contains(&hash) {
                    kinds.push(Kind::Head);
                }
                if tails.contains(&hash) {
                    kinds.push(Kind::Tail);
                }
                if reverses.contains(&hash) {
                    kinds.push(Kind::Reverse);
                }
                if centres.contains(&hash) {
                    kinds.push(Kind::CentreBrake);
                }
                if let Some(&(_, left)) = signals.iter().find(|(h, _)| *h == hash) {
                    kinds.push(Kind::Signal(left));
                }
                if reds.contains(&hash) {
                    kinds.push(Kind::Siren(true));
                }
                if blues.contains(&hash) {
                    kinds.push(Kind::Siren(false));
                }
                for (name, kind) in [
                    ("COPLIGHTBRIGHTRED", Kind::SirenBright(true)),
                    ("COPLIGHTBRIGHTBLUE", Kind::SirenBright(false)),
                    ("COPLIGHTWHITE", Kind::SirenWhite),
                    ("COPLIGHTORANGE", Kind::SirenOrange),
                ] {
                    if hash == bstring_hash(name) {
                        kinds.push(kind);
                    }
                }
                let frame = placement.transform * Mat4::from_cols_array(&marker.matrix);
                let at = frame.w_axis.truncate();
                let direction = frame.z_axis.truncate().normalize_or_zero();
                for kind in kinds {
                    if lamps.iter().any(|l| l.kind == kind && l.at.distance(at) < SAME_LAMP) {
                        continue;
                    }
                    lamps.push(Lamp { at, direction, kind, flags: marker_flags(hash) });
                }
            }
        }
        Self { lamps }
    }

    pub fn lamps(&self) -> &[Lamp] {
        &self.lamps
    }

    /// Only the optional rewrite indicators use generic circles; native lamps use authored flares.
    pub fn signal_glows(
        &self,
        to_world: Mat4,
        state: LampState,
        camera: Vec3,
        forward: Vec3,
        out: &mut Vec<EffectVertex>,
    ) {
        for lamp in self.lamps.iter().filter(|lamp| matches!(lamp.kind, Kind::Signal(_))) {
            let Some((radius, colour, brightness)) = look(lamp.kind, state) else { continue };
            geometry::glow(
                out,
                to_world.transform_point3(lamp.at),
                radius,
                geometry::color(colour, brightness),
                camera,
                forward,
            );
        }
    }

    #[cfg(test)]
    fn new(lamps: Vec<Lamp>) -> Self {
        Self { lamps }
    }
}

/// The optional indicator extension never substitutes for native head/brake/reverse/emergency lights.
fn look(kind: Kind, state: LampState) -> Option<(f32, [f32; 4], f32)> {
    let Kind::Signal(left) = kind else { return None };
    if !state.turn_signals {
        return None;
    }
    let steer = if state.steer.is_nan() { 0.0 } else { state.steer.clamp(-1.0, 1.0) };
    let held = if left { steer < -SIGNAL_STEER } else { steer > SIGNAL_STEER };
    let on = (state.time.max(0.0) % SIGNAL_PERIOD) < SIGNAL_PERIOD / 2.0;
    (held && on).then_some((SIGNAL_RADIUS, SIGNAL_COLOUR, SIGNAL_BRIGHTNESS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_lamps_never_receive_the_optional_generic_indicator_glows() {
        let state = LampState {
            turn_signals: true,
            steer: -1.0,
            headlights: true,
            brake: 1.0,
            reverse: true,
            time: 0.0,
            ..Default::default()
        };
        let lights = CarLights::new(
            [Kind::Head, Kind::Tail, Kind::CentreBrake, Kind::Reverse, Kind::Siren(true), Kind::Siren(false)]
                .map(|kind| Lamp { direction: Vec3::X, at: Vec3::ZERO, kind, flags: 0 })
                .to_vec(),
        );
        let mut out = Vec::new();
        lights.signal_glows(Mat4::IDENTITY, state, -Vec3::X * 10.0, Vec3::X, &mut out);
        assert!(out.is_empty(), "native lamps use installed flare and cop emitter profiles only");
    }

    #[test]
    fn optional_indicator_timing_and_direction_stay_independent_of_native_lamp_state() {
        let state = LampState { turn_signals: true, steer: -1.0, ..Default::default() };
        assert!(look(Kind::Signal(true), state).is_some());
        assert!(look(Kind::Signal(false), state).is_none());
        assert!(look(Kind::Signal(true), LampState { time: SIGNAL_PERIOD * 0.75, ..state }).is_none());
        assert!(look(Kind::Signal(true), LampState { turn_signals: false, ..state }).is_none());
    }
}
