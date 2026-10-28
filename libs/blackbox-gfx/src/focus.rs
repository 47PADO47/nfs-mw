//! Scene-supplied screen effects, independent of user grading: the speedbreaker vignette, the speed and
//! pursuit radial blur, and a scene grade (a darkening tinted vignette, a colour multiplier and a contrast).

/// A grade of the finished frame: the edge of the screen goes towards `edge`, every colour is multiplied by `tint`, and
/// the contrast moves about mid grey. The scene mixes the values with its own level, so the neutral grade (the default)
/// leaves the frame as it is and draws no pass. A backend without the grade ignores it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenGrade {
    /// What the frame is multiplied by at the corners of the screen, per channel (1 leaves it).
    pub edge: [f32; 3],
    /// Where the edge starts to darken: the distance from the centre, 0 at the centre and 1 at a corner.
    pub inner: f32,
    /// What every colour is multiplied by, per channel.
    pub tint: [f32; 3],
    /// Contrast about mid grey: 1 leaves the frame, above 1 pushes it apart.
    pub contrast: f32,
}

impl Default for ScreenGrade {
    fn default() -> Self {
        Self { edge: [1.0; 3], inner: 0.5, tint: [1.0; 3], contrast: 1.0 }
    }
}

impl ScreenGrade {
    /// Whether the grade changes nothing (the `inner` distance does not matter then).
    pub fn is_neutral(&self) -> bool {
        self.edge == [1.0; 3] && self.tint == [1.0; 3] && self.contrast == 1.0
    }

    pub fn sanitized(self) -> Self {
        let pick = |v: f32, fallback: f32, max: f32| match v.is_finite() {
            true => v.clamp(0.0, max),
            false => fallback,
        };
        let colour = |c: [f32; 3]| [pick(c[0], 1.0, 4.0), pick(c[1], 1.0, 4.0), pick(c[2], 1.0, 4.0)];
        Self {
            edge: colour(self.edge),
            inner: pick(self.inner, 0.5, 1.0),
            tint: colour(self.tint),
            contrast: pick(self.contrast, 1.0, 4.0),
        }
    }
}

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
    /// The scene's colour grade and darkening vignette; the neutral default draws nothing.
    pub grade: ScreenGrade,
}

impl Default for FocusFilter {
    fn default() -> Self {
        Self { vignette: 0.0, radial_blur: 0.0, radial_centre: [0.5, 0.5], grade: ScreenGrade::default() }
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
            grade: self.grade.sanitized(),
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
        let dirty = FocusFilter { radial_blur: 4.0, radial_centre: [-1.0, f32::INFINITY], ..Default::default() };
        let clean = dirty.sanitized();
        assert_eq!(clean.radial_blur, 1.0);
        assert_eq!(clean.radial_centre, [0.0, 0.5]);
        let nan = FocusFilter { radial_blur: f32::NAN, ..Default::default() }.sanitized();
        assert_eq!(nan.radial_blur, 0.0);
    }

    #[test]
    fn the_default_grade_is_neutral_and_a_dirty_one_is_cleaned() {
        assert!(ScreenGrade::default().is_neutral());
        assert!(FocusFilter::default().grade.is_neutral());
        let tinted = ScreenGrade { tint: [1.0, 0.9, 1.0], ..Default::default() };
        assert!(!tinted.is_neutral());
        let dirty = ScreenGrade { edge: [9.0, f32::NAN, -1.0], inner: 3.0, tint: [1.0; 3], contrast: f32::INFINITY };
        let clean = dirty.sanitized();
        assert_eq!(clean.edge, [4.0, 1.0, 0.0]);
        assert_eq!((clean.inner, clean.contrast), (1.0, 1.0));
    }
}
