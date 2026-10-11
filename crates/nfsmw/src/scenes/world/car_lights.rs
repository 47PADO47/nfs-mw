//! The lamps of a car drawn as glows: the headlights and the tail and brake lamps, at the light markers of the car's
//! solids (`LEFT_HEADLIGHT`, `LEFT_BRAKELIGHT`, `BRAKELIGHT_CENTRE`, …). The stock lamp textures are fixed and only
//! the brake lamp has an `_ON` variant (`docs/specs/car-assembly.md` §7), so the light itself is a camera-facing
//! additive glow in the effect layer, at the marker.

use blackbox_hash::bstring_hash;
use blackbox_render::EffectVertex;
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;

use super::vehicle_effects::geometry;

const HEAD_MARKERS: [&str; 2] = ["LEFT_HEADLIGHT", "RIGHT_HEADLIGHT"];
const TAIL_MARKERS: [&str; 3] = ["LEFT_BRAKELIGHT", "RIGHT_BRAKELIGHT", "BRAKELIGHT_CENTRE"];
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Head,
    Tail,
}

/// One lamp: its place in the car model's frame, metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lamp {
    pub at: Vec3,
    pub kind: Kind,
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
        let mut lamps: Vec<Lamp> = Vec::new();
        for placement in &model.placements {
            let Some(solid) = model.solids.get(&placement.solid) else { continue };
            for marker in &solid.markers {
                let kind = match () {
                    _ if heads.contains(&marker.name_hash) => Kind::Head,
                    _ if tails.contains(&marker.name_hash) => Kind::Tail,
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

    /// Appends the glows of the lamps of a car whose model frame is `to_world`. `brake` (0 to 1) lights the brake
    /// lamps; NaN counts as released.
    pub fn glows(&self, to_world: Mat4, brake: f32, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for lamp in &self.lamps {
            let (radius, colour, brightness) = look(lamp.kind, brake);
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

/// Radius (metres), colour and brightness of a lamp's glow given the brake.
fn look(kind: Kind, brake: f32) -> (f32, [f32; 4], f32) {
    let brake = if brake.is_nan() { 0.0 } else { brake.clamp(0.0, 1.0) };
    match kind {
        Kind::Head => (HEAD_RADIUS, HEAD_COLOUR, HEAD_BRIGHTNESS),
        Kind::Tail => {
            let lit = TAIL_REST + (TAIL_BRAKE - TAIL_REST) * brake;
            (TAIL_RADIUS + (TAIL_BRAKE_RADIUS - TAIL_RADIUS) * brake, TAIL_COLOUR, lit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn braking_brightens_and_widens_only_the_tail_lamps() {
        let (r0, _, b0) = look(Kind::Tail, 0.0);
        let (r1, _, b1) = look(Kind::Tail, 1.0);
        assert!(b1 > b0 && r1 > r0);
        assert_eq!(look(Kind::Head, 0.0), look(Kind::Head, 1.0));
        assert_eq!(look(Kind::Tail, f32::NAN).2, TAIL_REST, "NaN counts as released");
    }

    #[test]
    fn every_lamp_adds_one_quad() {
        let lights = CarLights::new(vec![
            Lamp { at: Vec3::new(2.0, 0.5, 0.5), kind: Kind::Head },
            Lamp { at: Vec3::new(-2.0, 0.5, 0.5), kind: Kind::Tail },
        ]);
        let mut out = Vec::new();
        lights.glows(Mat4::IDENTITY, 0.0, Vec3::new(-10.0, 0.0, 1.0), Vec3::X, &mut out);
        assert_eq!(out.len(), 12);
    }
}
