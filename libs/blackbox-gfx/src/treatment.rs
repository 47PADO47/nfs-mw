//! Parameters for a display-space colour treatment supplied by the game.

/// Two cubic Bezier curves modulate the desaturated source and a tinted addition.
/// Curve values are ordinates; the argument is clamped blurred luminance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourTreatment {
    pub tint: [f32; 3],
    pub luminance: [f32; 3],
    /// Historical name: source-colour weight (0 = grayscale, 1 = source colour).
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

/// Optional replacements for a loaded profile; its curves and luminance stay authored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourTreatmentOverride {
    pub tint: [f32; 3],
    /// Historical name: source-colour weight (0 = grayscale, 1 = source colour).
    pub desaturation: f32,
    pub black_intensity: f32,
    pub colour_intensity: f32,
}
impl ColourTreatmentOverride {
    pub fn is_valid(&self) -> bool {
        self.tint.iter().all(|v| v.is_finite() && v.abs() <= 16.0)
            && (0.0..=1.0).contains(&self.desaturation)
            && (0.0..=4.0).contains(&self.black_intensity)
            && (0.0..=4.0).contains(&self.colour_intensity)
    }

    /// Invalid replacements leave the loaded profile unchanged.
    pub fn apply(self, profile: ColourTreatment) -> ColourTreatment {
        if !self.is_valid() {
            return profile;
        }
        ColourTreatment {
            tint: self.tint,
            desaturation: self.desaturation,
            black_intensity: self.black_intensity,
            colour_intensity: self.colour_intensity,
            ..profile
        }
    }
}

impl ColourTreatment {
    /// This profile towards `to` by `weight` (0 gives this one, 1 gives `to`), every value alike.
    pub fn lerp(&self, to: &Self, weight: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * weight;
        let mix3 = |a: [f32; 3], b: [f32; 3]| std::array::from_fn(|i| mix(a[i], b[i]));
        let mix4 = |a: [f32; 4], b: [f32; 4]| std::array::from_fn(|i| mix(a[i], b[i]));
        Self {
            tint: mix3(self.tint, to.tint),
            luminance: mix3(self.luminance, to.luminance),
            desaturation: mix(self.desaturation, to.desaturation),
            black_intensity: mix(self.black_intensity, to.black_intensity),
            colour_intensity: mix(self.colour_intensity, to.colour_intensity),
            black_curve: mix4(self.black_curve, to.black_curve),
            colour_curve: mix4(self.colour_curve, to.colour_curve),
        }
    }
}

/// A per-frame change of the scene's colour profile: the loaded profile (with its replacements) blended towards `target`
/// by `weight`, and the graded colour multiplied by `brightness`. The default changes nothing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColourLook {
    /// The look blended towards; `None` keeps the loaded profile.
    pub target: Option<ColourTreatment>,
    /// 0 keeps the loaded profile, 1 is the target as it is.
    pub weight: f32,
    /// What the graded colour is multiplied by.
    pub brightness: f32,
}

impl Default for ColourLook {
    fn default() -> Self {
        Self { target: None, weight: 0.0, brightness: 1.0 }
    }
}

impl ColourLook {
    /// A look a shader can take: an invalid target is dropped, the weight is clamped to 0..1 and the brightness to
    /// 0..16 (1 when it is not finite).
    pub fn sanitized(self) -> Self {
        let weight = match self.weight.is_finite() {
            true => self.weight.clamp(0.0, 1.0),
            false => 0.0,
        };
        let brightness = match self.brightness.is_finite() {
            true => self.brightness.clamp(0.0, 16.0),
            false => 1.0,
        };
        Self { target: self.target.filter(ColourTreatment::is_valid), weight, brightness }
    }

    /// `base` with this look's target blended in.
    pub fn grade(&self, base: ColourTreatment) -> ColourTreatment {
        match self.target {
            Some(target) if self.weight >= 1.0 => target,
            Some(target) if self.weight > 0.0 => base.lerp(&target, self.weight),
            _ => base,
        }
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;

    fn flat(value: f32) -> ColourTreatment {
        ColourTreatment {
            tint: [value; 3],
            luminance: [value; 3],
            desaturation: value,
            black_intensity: value,
            colour_intensity: value,
            black_curve: [value; 4],
            colour_curve: [value; 4],
        }
    }

    #[test]
    fn a_look_blends_from_the_base_and_holds_its_ends() {
        let look = |weight| ColourLook { target: Some(flat(1.0)), weight, brightness: 1.0 };
        assert_eq!(look(0.0).grade(flat(0.0)), flat(0.0));
        assert_eq!(look(0.25).grade(flat(0.0)), flat(0.25));
        assert_eq!(look(1.0).grade(flat(0.0)), flat(1.0), "full weight is the target as it is");
        assert_eq!(ColourLook::default().grade(flat(0.5)), flat(0.5));
    }

    #[test]
    fn a_sanitized_look_is_one_a_shader_can_take() {
        let wild = ColourLook { target: Some(flat(1.0)), weight: 7.0, brightness: f32::INFINITY }.sanitized();
        assert_eq!((wild.weight, wild.brightness), (1.0, 1.0));
        let invalid = ColourLook { target: Some(flat(9.0)), weight: f32::NAN, brightness: -2.0 }.sanitized();
        assert_eq!(invalid, ColourLook { target: None, weight: 0.0, brightness: 0.0 });
    }
}
