//! Scene-supplied screen effects, independent of user grading: the speedbreaker vignette and the speed and
//! pursuit radial blur.

/// The screen effects a scene asks for. The default asks for none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusFilter {
    /// Strength of the additive vignette of the speedbreaker, 0 to 1.
    pub vignette: f32,
    /// Strength of the radial blur of the high-speed and pursuit effect, 0 to 1. Zero draws no blur, and a
    /// backend without the blur (see `Capabilities::radial_blur`) ignores it.
    pub radial_blur: f32,
    /// The centre of the radial blur in UV (0 to 1 on both axes): the middle of the screen.
    pub radial_centre: [f32; 2],
}

impl Default for FocusFilter {
    fn default() -> Self {
        Self { vignette: 0.0, radial_blur: 0.0, radial_centre: [0.5, 0.5] }
    }
}

impl FocusFilter {
    pub fn sanitized(self) -> Self {
        let pick = |v: f32, fallback: f32, max: f32| match v.is_finite() {
            true => v.clamp(0.0, max),
            false => fallback,
        };
        let centre = |v: f32| pick(v, 0.5, 1.0);
        Self {
            vignette: pick(self.vignette, 0.0, 1.0),
            radial_blur: pick(self.radial_blur, 0.0, 1.0),
            radial_centre: [centre(self.radial_centre[0]), centre(self.radial_centre[1])],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_asks_for_nothing_and_centres_the_blur() {
        let d = FocusFilter::default();
        assert_eq!((d.vignette, d.radial_blur), (0.0, 0.0));
        assert_eq!(d.radial_centre, [0.5, 0.5]);
    }

    #[test]
    fn the_blur_is_clamped_and_its_centre_is_kept_on_screen() {
        let dirty = FocusFilter { vignette: 0.0, radial_blur: 4.0, radial_centre: [-1.0, f32::INFINITY] };
        let clean = dirty.sanitized();
        assert_eq!(clean.radial_blur, 1.0);
        assert_eq!(clean.radial_centre, [0.0, 0.5]);
        let nan = FocusFilter { radial_blur: f32::NAN, ..Default::default() }.sanitized();
        assert_eq!(nan.radial_blur, 0.0);
    }
}
