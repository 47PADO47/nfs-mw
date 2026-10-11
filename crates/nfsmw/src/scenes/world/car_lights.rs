//! The lamps of a car drawn as glows: the headlights, the tail and brake lamps and the reverse lamps, at the light
//! markers of the car's solids (`LEFT_HEADLIGHT`, `LEFT_BRAKELIGHT`, `BRAKELIGHT_CENTRE`, `LEFT_REVERSE`, …). The stock
//! lamp textures are fixed and only the brake lamp has an `_ON` variant (`docs/specs/car-assembly.md` §7), so the
//! light itself is a camera-facing additive glow in the effect layer, at the marker.

use blackbox_hash::bstring_hash;
use blackbox_render::EffectVertex;
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;

use super::vehicle_effects::geometry;

const HEAD_MARKERS: [&str; 2] = ["LEFT_HEADLIGHT", "RIGHT_HEADLIGHT"];
const TAIL_MARKERS: [&str; 3] = ["LEFT_BRAKELIGHT", "RIGHT_BRAKELIGHT", "BRAKELIGHT_CENTRE"];
const REVERSE_MARKERS: [&str; 2] = ["LEFT_REVERSE", "RIGHT_REVERSE"];
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Head,
    Tail,
    Reverse,
}

/// One lamp: its place in the car model's frame, metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lamp {
    pub at: Vec3,
    pub kind: Kind,
}

/// What lights the lamps of a car: the brake pedal (0 to 1) and whether the car is in reverse.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LampState {
    pub brake: f32,
    pub reverse: bool,
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
        let mut lamps: Vec<Lamp> = Vec::new();
        for placement in &model.placements {
            let Some(solid) = model.solids.get(&placement.solid) else { continue };
            for marker in &solid.markers {
                let kind = match () {
                    _ if heads.contains(&marker.name_hash) => Kind::Head,
                    _ if tails.contains(&marker.name_hash) => Kind::Tail,
                    _ if reverses.contains(&marker.name_hash) => Kind::Reverse,
                    _ => continue,
                };
                let at = (placement.transform * Mat4::from_cols_array(&marker.matrix)).w_axis.truncate();
                if lamps.iter().any(|l| l.kind == kind && l.at.distance(at) < SAME_LAMP) {
                    continue;
                }
                lamps.push(Lamp { at, kind });
            }
        }
        Self { lamps }
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
    // NaN counts as released.
    let brake = if state.brake.is_nan() { 0.0 } else { state.brake.clamp(0.0, 1.0) };
    match kind {
        Kind::Head => Some((HEAD_RADIUS, HEAD_COLOUR, HEAD_BRIGHTNESS)),
        Kind::Tail => {
            let lit = TAIL_REST + (TAIL_BRAKE - TAIL_REST) * brake;
            Some((TAIL_RADIUS + (TAIL_BRAKE_RADIUS - TAIL_RADIUS) * brake, TAIL_COLOUR, lit))
        }
        Kind::Reverse => state.reverse.then_some((REVERSE_RADIUS, REVERSE_COLOUR, REVERSE_BRIGHTNESS)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASED: LampState = LampState { brake: 0.0, reverse: false };

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
}
