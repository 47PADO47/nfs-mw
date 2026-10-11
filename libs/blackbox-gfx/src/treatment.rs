//! Parameters for a display-space colour treatment supplied by the game.

/// Two cubic Bezier curves modulate the desaturated source and a tinted addition.
/// Curve values are ordinates; the argument is clamped blurred luminance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourTreatment {
    pub tint: [f32; 3],
    pub luminance: [f32; 3],
    pub desaturation: f32,
    pub black_intensity: f32,
    pub colour_intensity: f32,
    pub black_curve: [f32; 4],
    pub colour_curve: [f32; 4],
}

impl ColourTreatment {
    /// Reject invalid external data before it becomes a shader uniform.
    pub fn is_valid(&self) -> bool {
        self.tint
            .iter()
            .chain(&self.luminance)
            .chain(&self.black_curve)
            .chain(&self.colour_curve)
            .all(|v| v.is_finite() && v.abs() <= 16.0)
            && (0.0..=1.0).contains(&self.desaturation)
            && (0.0..=4.0).contains(&self.black_intensity)
            && (0.0..=4.0).contains(&self.colour_intensity)
    }
}
