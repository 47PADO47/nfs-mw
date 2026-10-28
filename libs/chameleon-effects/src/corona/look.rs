//! What a corona layer looks like from a given angle and distance: the executable's chain of curves, evaluated.
//! Evidence and the parts that are not confirmed: `docs/formats/chameleon-corona.md`.
//!
//! The executable first runs the corona's own curve block on `(angle, distance, distance, distance)`: lane 0 is how visible
//! the lamp is at that angle off its axis, lane 1 a size factor by distance, lane 2 a brightness by distance. The product of
//! the angle and brightness lanes feeds the layer's own curve block on `(i, i, i, distance)`, whose four outputs are the
//! sprite's two sizes, its colour intensity and a distance factor on the size.

use super::{Corona, Layer, LayerKind};

/// The numbers of a lamp's record that the layers scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecordFactors {
    /// The record's intensity factor (+0x34).
    pub intensity: f32,
    /// The record's size factors (+0x38, +0x3C).
    pub size: [f32; 2],
    /// The record's colour factor.
    pub tint: [f32; 4],
}

impl Default for RecordFactors {
    fn default() -> Self {
        Self { intensity: 1.0, size: [1.0; 2], tint: [1.0; 4] }
    }
}

/// A layer seen from one place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seen {
    /// The sprite's width and height in the executable's units (taken as metres **[unconfirmed]**).
    pub size: [f32; 2],
    /// Red, green and blue after the layer's colour, the record's colour factor and the intensities.
    pub colour: [f32; 3],
    /// The layer's alpha times the record's.
    pub alpha: f32,
    /// The angle visibility lane (1 on the axis).
    pub visibility: f32,
}

impl Corona {
    /// The corona's own four lanes for an angle off the lamp's axis, radians, and a distance, metres.
    pub fn lanes(&self, angle: f32, distance: f32) -> [f32; 4] {
        self.curves.eval([angle, distance, distance, distance])
    }

    /// How `layer` looks at `angle` radians off the lamp's axis and `distance` metres away, for a lamp record with `factors`.
    pub fn seen(&self, layer: &Layer, factors: &RecordFactors, angle: f32, distance: f32) -> Seen {
        let lanes = self.lanes(angle, distance);
        let brightness = lanes[2] * lanes[0];
        let out = layer.curves.eval([brightness, brightness, brightness, distance]);
        let size_factor = lanes[1];
        // A glow's size is scaled by the distance lane of its own curves; a flare's and a beam's are not.
        let scale = match layer.kind {
            LayerKind::Glow => out[3],
            LayerKind::Flare | LayerKind::Beam => 1.0,
        };
        let size = [size_factor * factors.size[0] * out[0] * scale, size_factor * factors.size[1] * out[1] * scale];
        let strength = out[2] * factors.intensity;
        let colour = [0, 1, 2].map(|i| (layer.colour[i] * factors.tint[i] * strength).max(0.0));
        Seen { size, colour, alpha: layer.colour[3] * factors.tint[3], visibility: lanes[0] }
    }

    /// The exponent `p` for which `cos(angle)^p` best matches the corona's visibility lane from the axis to 60 degrees off it,
    /// or `None` when the lamp is not visible on its axis (a beam seen from the side).
    pub fn falloff_power(&self) -> Option<f32> {
        let on_axis = self.lanes(0.0, 10.0)[0];
        if on_axis <= 0.01 {
            return None;
        }
        let off = self.lanes(60f32.to_radians(), 10.0)[0] / on_axis;
        if off <= 0.0 {
            return Some(MAX_POWER);
        }
        Some((off.ln() / 0.5f32.ln()).clamp(0.0, MAX_POWER))
    }
}

/// The largest falloff exponent [`Corona::falloff_power`] gives.
pub const MAX_POWER: f32 = 8.0;
