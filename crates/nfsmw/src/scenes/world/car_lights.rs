//! The lamps of a car drawn as glows, at the light markers of the car's solids: the headlights, the tail and brake
//! lamps, the reverse lamps, the turn signals and the cop light bars. The stock lamp textures are fixed and only the
//! brake lamp has an `_ON` variant (`docs/specs/car-assembly.md` §7), so the light itself is a camera-facing additive
//! glow in the effect layer, at the marker.
//!
//! The turn signals and the cop bars are approximations: they blink at fixed rates from the steering input and the
//! cop's role, not from the original's lamp controller, which is not decoded yet. The signals reuse the head and tail
//! markers of their side, since the car data has no signal markers.

use blackbox_hash::bstring_hash;
use blackbox_render::{EffectLayer, EffectVertex};
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;

use super::vehicle_effects::geometry;

const HEAD_MARKERS: [&str; 2] = ["LEFT_HEADLIGHT", "RIGHT_HEADLIGHT"];
const TAIL_MARKERS: [&str; 4] = ["LEFT_BRAKELIGHT", "RIGHT_BRAKELIGHT", "BRAKELIGHT_CENTRE", "CENTRE_BRAKELIGHT"];
const REVERSE_MARKERS: [&str; 2] = ["LEFT_REVERSE", "RIGHT_REVERSE"];
/// The side (true for left) of each head and tail lamp a turn signal blinks from.
const SIGNAL_MARKERS: [(&str, bool); 4] =
    [("LEFT_HEADLIGHT", true), ("RIGHT_HEADLIGHT", false), ("LEFT_BRAKELIGHT", true), ("RIGHT_BRAKELIGHT", false)];
/// The cop light bar: red lamps and blue lamps.
const SIREN_RED: [&str; 2] = ["COPLIGHTRED", "COPLIGHTBRIGHTRED"];
const SIREN_BLUE: [&str; 2] = ["COPLIGHTBLUE", "COPLIGHTBRIGHTBLUE"];
/// Two markers closer than this (metres) are the same lamp, seen in two solids.
const SAME_LAMP: f32 = 0.01;
/// Glow radius (metres) and brightness, 0 to 1, of a headlight: on all the time.
const HEAD_RADIUS: f32 = 0.45;
const HEAD_BRIGHTNESS: f32 = 0.8;
const HEAD_COLOUR: [f32; 4] = [1.0, 0.95, 0.8, 1.0];
/// A tail lamp glows dimly and brightens and widens with the brake.
const TAIL_RADIUS: f32 = 0.25;
const TAIL_BRAKE_RADIUS: f32 = 0.4;
const TAIL_REST: f32 = 0.4;
const TAIL_BRAKE: f32 = 1.0;
const TAIL_COLOUR: [f32; 4] = [1.0, 0.08, 0.04, 1.0];
/// A reverse lamp glows white while the car is in reverse, and is dark otherwise.
const REVERSE_RADIUS: f32 = 0.3;
const REVERSE_BRIGHTNESS: f32 = 0.8;
const REVERSE_COLOUR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
/// Turn signal: amber, blinking 1.5 times a second while the steering is held past `SIGNAL_STEER`.
const SIGNAL_RADIUS: f32 = 0.3;
const SIGNAL_BRIGHTNESS: f32 = 0.9;
const SIGNAL_COLOUR: [f32; 4] = [1.0, 0.55, 0.05, 1.0];
const SIGNAL_STEER: f32 = 0.3;
const SIGNAL_PERIOD: f32 = 1.0 / 1.5;
/// Cop bar: red and blue, alternating four times a second while the cop is on.
const SIREN_RADIUS: f32 = 0.3;
const SIREN_BRIGHTNESS: f32 = 0.9;
const SIREN_RED_COLOUR: [f32; 4] = [1.0, 0.05, 0.05, 1.0];
const SIREN_BLUE_COLOUR: [f32; 4] = [0.1, 0.3, 1.0, 1.0];
const SIREN_PERIOD: f32 = 0.25;
/// Headlight beam: how far ahead, how wide at its end and how bright (metres and 0 to 1), from the lamp.
const BEAM_LENGTH: f32 = 14.0;
const BEAM_WIDTH: f32 = 3.5;
const BEAM_NEAR: f32 = 0.15;
const BEAM_BRIGHTNESS: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Head,
    Tail,
    Reverse,
    /// A turn signal on the left (true) or right side.
    Signal(bool),
    /// A cop light bar lamp: red (true) or blue.
    Siren(bool),
}

/// One lamp: its place in the car model's frame, metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lamp {
    pub at: Vec3,
    pub kind: Kind,
}

/// What lights the lamps of a car at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LampState {
    /// The brake pedal, 0 to 1.
    pub brake: f32,
    pub reverse: bool,
    /// The steering, -1 to 1; positive steers left, so the left signals blink.
    pub steer: f32,
    /// The headlights are on.
    pub headlights: bool,
    /// The cop light bar is on.
    pub siren: bool,
    /// Simulated seconds, for the blinking.
    pub time: f32,
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
                if let Some(&(_, left)) = signals.iter().find(|(h, _)| *h == hash) {
                    kinds.push(Kind::Signal(left));
                }
                if reds.contains(&hash) {
                    kinds.push(Kind::Siren(true));
                }
                if blues.contains(&hash) {
                    kinds.push(Kind::Siren(false));
                }
                let at = (placement.transform * Mat4::from_cols_array(&marker.matrix)).w_axis.truncate();
                for kind in kinds {
                    if lamps.iter().any(|l| l.kind == kind && l.at.distance(at) < SAME_LAMP) {
                        continue;
                    }
                    lamps.push(Lamp { at, kind });
                }
            }
        }
        Self { lamps }
    }

    /// Appends a faint additive beam ahead of each headlight, as flat streaks at the lamp's height, for a car whose
    /// model frame is `to_world`. An approximation of the original's projected headlight cone.
    pub fn beams(&self, to_world: Mat4, headlights: bool, out: &mut Vec<EffectVertex>) {
        if !headlights {
            return;
        }
        let colour = geometry::color(HEAD_COLOUR, BEAM_BRIGHTNESS);
        for lamp in self.lamps.iter().filter(|l| l.kind == Kind::Head) {
            let at = lamp.at;
            let side = Vec3::Y * BEAM_NEAR;
            let far = Vec3::X * BEAM_LENGTH;
            let spread = Vec3::Y * BEAM_WIDTH;
            let corners = [at + side, at + far + spread, at + far - spread, at - side];
            let world = corners.map(|c| to_world.transform_point3(c));
            EffectLayer::quad(out, world, colour);
        }
    }

    /// Appends the glows of the lit lamps of a car whose model frame is `to_world`.
    pub fn glows(&self, to_world: Mat4, state: LampState, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for lamp in &self.lamps {
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

/// Radius (metres), colour and brightness of a lamp's glow given the state, or `None` when it is dark.
fn look(kind: Kind, state: LampState) -> Option<(f32, [f32; 4], f32)> {
    // NaN counts as released or straight.
    let brake = if state.brake.is_nan() { 0.0 } else { state.brake.clamp(0.0, 1.0) };
    let steer = if state.steer.is_nan() { 0.0 } else { state.steer.clamp(-1.0, 1.0) };
    match kind {
        Kind::Head => state.headlights.then_some((HEAD_RADIUS, HEAD_COLOUR, HEAD_BRIGHTNESS)),
        Kind::Tail => {
            let lit = TAIL_REST + (TAIL_BRAKE - TAIL_REST) * brake;
            Some((TAIL_RADIUS + (TAIL_BRAKE_RADIUS - TAIL_RADIUS) * brake, TAIL_COLOUR, lit))
        }
        Kind::Reverse => state.reverse.then_some((REVERSE_RADIUS, REVERSE_COLOUR, REVERSE_BRIGHTNESS)),
        Kind::Signal(left) => {
            let held = if left { steer > SIGNAL_STEER } else { steer < -SIGNAL_STEER };
            let on = (state.time.max(0.0) % SIGNAL_PERIOD) < SIGNAL_PERIOD / 2.0;
            (held && on).then_some((SIGNAL_RADIUS, SIGNAL_COLOUR, SIGNAL_BRIGHTNESS))
        }
        Kind::Siren(red) => {
            let red_phase = ((state.time.max(0.0) / SIREN_PERIOD) as u64).is_multiple_of(2);
            let lit = state.siren && red_phase == red;
            let colour = if red { SIREN_RED_COLOUR } else { SIREN_BLUE_COLOUR };
            lit.then_some((SIREN_RADIUS, colour, SIREN_BRIGHTNESS))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASED: LampState =
        LampState { brake: 0.0, reverse: false, steer: 0.0, headlights: true, siren: false, time: 0.0 };

    #[test]
    fn braking_brightens_and_widens_only_the_tail_lamps() {
        let (r0, _, b0) = look(Kind::Tail, RELEASED).unwrap();
        let braking = LampState { brake: 1.0, ..RELEASED };
        let (r1, _, b1) = look(Kind::Tail, braking).unwrap();
        assert!(b1 > b0 && r1 > r0);
        assert_eq!(look(Kind::Head, RELEASED), look(Kind::Head, braking));
        let nan = LampState { brake: f32::NAN, ..RELEASED };
        assert_eq!(look(Kind::Tail, nan).unwrap().2, TAIL_REST, "NaN counts as released");
    }

    #[test]
    fn reverse_lamps_are_lit_only_in_reverse() {
        assert!(look(Kind::Reverse, RELEASED).is_none());
        assert!(look(Kind::Reverse, LampState { reverse: true, ..RELEASED }).is_some());
    }

    #[test]
    fn a_turn_signal_blinks_only_on_its_side_while_steering() {
        let left_turn = LampState { steer: 0.6, ..RELEASED };
        assert!(look(Kind::Signal(true), LampState { time: 0.1, ..left_turn }).is_some());
        assert!(look(Kind::Signal(true), LampState { time: 0.5, ..left_turn }).is_none(), "the off half");
        assert!(look(Kind::Signal(false), LampState { time: 0.1, ..left_turn }).is_none(), "the other side");
        assert!(look(Kind::Signal(true), LampState { time: 0.1, ..RELEASED }).is_none(), "straight ahead");
    }

    #[test]
    fn the_cop_bar_alternates_red_and_blue_only_while_on() {
        let on = LampState { siren: true, ..RELEASED };
        assert!(look(Kind::Siren(true), LampState { time: 0.05, ..on }).is_some());
        assert!(look(Kind::Siren(false), LampState { time: 0.05, ..on }).is_none());
        assert!(look(Kind::Siren(false), LampState { time: 0.3, ..on }).is_some());
        assert!(look(Kind::Siren(true), LampState { time: 0.05, ..RELEASED }).is_none(), "off");
    }

    #[test]
    fn every_lit_lamp_adds_one_quad() {
        let lights = CarLights::new(vec![
            Lamp { at: Vec3::new(2.0, 0.5, 0.5), kind: Kind::Head },
            Lamp { at: Vec3::new(-2.0, 0.5, 0.5), kind: Kind::Tail },
            Lamp { at: Vec3::new(-2.0, 0.2, 0.5), kind: Kind::Reverse },
        ]);
        let camera = Vec3::new(-10.0, 0.0, 1.0);
        let mut out = Vec::new();
        lights.glows(Mat4::IDENTITY, RELEASED, camera, Vec3::X, &mut out);
        assert_eq!(out.len(), 12, "the reverse lamp is dark");
        out.clear();
        lights.glows(Mat4::IDENTITY, LampState { reverse: true, ..RELEASED }, camera, Vec3::X, &mut out);
        assert_eq!(out.len(), 18);
    }

    #[test]
    fn each_headlight_casts_one_beam_quad() {
        let lights = CarLights::new(vec![
            Lamp { at: Vec3::new(2.0, 0.5, 0.5), kind: Kind::Head },
            Lamp { at: Vec3::new(2.0, -0.5, 0.5), kind: Kind::Head },
            Lamp { at: Vec3::new(-2.0, 0.5, 0.5), kind: Kind::Tail },
        ]);
        let mut out = Vec::new();
        lights.beams(Mat4::IDENTITY, true, &mut out);
        assert_eq!(out.len(), 12);
    }

    #[test]
    fn headlights_off_darkens_the_beams_and_the_head_glows_only() {
        let lights = CarLights::new(vec![
            Lamp { at: Vec3::new(2.0, 0.5, 0.5), kind: Kind::Head },
            Lamp { at: Vec3::new(-2.0, 0.5, 0.5), kind: Kind::Tail },
        ]);
        let camera = Vec3::new(-10.0, 0.0, 1.0);
        let mut on = Vec::new();
        lights.glows(Mat4::IDENTITY, RELEASED, camera, Vec3::X, &mut on);
        let mut off = Vec::new();
        lights.glows(Mat4::IDENTITY, LampState { headlights: false, ..RELEASED }, camera, Vec3::X, &mut off);
        assert_eq!(on.len() - off.len(), 6, "one headlight glow, six vertices");
        let mut beams = Vec::new();
        lights.beams(Mat4::IDENTITY, false, &mut beams);
        assert!(beams.is_empty());
    }
}
