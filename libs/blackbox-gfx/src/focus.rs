//! Scene-supplied additive screen-mask strength, independent of user grading.

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct FocusFilter {
    pub vignette: f32,
}

impl FocusFilter {
    pub fn sanitized(self) -> Self {
        let pick = |v: f32, fallback: f32, max: f32| match v.is_finite() {
            true => v.clamp(0.0, max),
            false => fallback,
        };
        Self { vignette: pick(self.vignette, 0.0, 1.0) }
    }
}
