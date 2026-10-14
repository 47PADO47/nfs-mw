//! Retained world-time target, independent of the real-time camera zoom.
use blackbox_gfx::{FocusFilter, RenderBackend, RgbaImage};
use nfsmw_data::visual_treatment::speedbreaker::Profile;

pub(super) struct Presentation {
    profile: Option<Profile>,
    mask: Option<RgbaImage>,
    active: bool,
    elapsed: f64,
    current: f32,
    transitioning: bool,
}

impl Presentation {
    pub fn new(profile: Option<Profile>, mask: Option<RgbaImage>) -> Self {
        if profile.is_none() || mask.is_none() {
            log::warn!("Speedbreaker visual profile or vignette unavailable; treatment disabled");
        }
        Self { profile, mask, active: false, elapsed: 0.0, current: 0.0, transitioning: false }
    }

    pub fn init(&self, renderer: &mut dyn RenderBackend) {
        renderer.set_focus_mask(self.mask.as_ref().filter(|_| self.profile.is_some()));
        renderer.set_focus_filter(FocusFilter::default());
    }

    pub fn update(&mut self, active: bool, world_dt: f32) -> FocusFilter {
        let Some(p) = self.profile else { return FocusFilter::default() };
        if !world_dt.is_finite() || world_dt <= 0.0 {
            return self.filter(p);
        }
        let changed = active != self.active;
        if changed {
            self.active = active;
            self.elapsed = 0.0;
            self.transitioning = true;
        }
        if !self.transitioning {
            return self.filter(p);
        }
        if !changed {
            self.elapsed += f64::from(world_dt);
        }
        let target = f32::from(active);
        if self.elapsed > f64::from(p.seconds) {
            self.current = target;
            self.transitioning = false;
            return self.filter(p);
        }
        // Original target reversal restarts the authored graph. It does not
        // interpolate from the prior output on interrupted entry/exit.
        let mut t = (self.elapsed / f64::from(p.seconds)) as f32;
        if self.current > target {
            t = 1.0 - t;
        }
        let u = 1.0 - t;
        self.current = u * u * u * p.curve[0]
            + 3.0 * t * u * u * p.curve[1]
            + 3.0 * t * t * u * p.curve[2]
            + t * t * t * p.curve[3];
        self.filter(p)
    }

    fn filter(&self, _p: Profile) -> FocusFilter {
        FocusFilter { vignette: self.current.clamp(0.0, 1.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_seconds_and_pauses_preserve_authored_target_at_all_render_rates() {
        let p = Profile { seconds: 0.8, curve: [0.0, 0.4, 0.76, 1.0], radial_scale: 1.5, radial_offset: 0.015 };
        for fps in [5, 30, 60, 120, 144, 240, 360] {
            let mut effect = Presentation::new(Some(p), None);
            assert_eq!(effect.update(false, 1.0).vignette, 0.0, "inactive starts neutral");
            assert_eq!(effect.update(true, 0.01).vignette, 0.0, "target changed at current world time");
            for _ in 0..fps {
                effect.update(true, 1.0 / fps as f32);
            }
            assert_eq!(effect.current, 1.0, "hold at{fps}");
            assert_eq!(effect.update(true, 0.0).vignette, 1.0);
            assert_eq!(effect.update(true, f32::NAN).vignette, 1.0);
            effect.update(false, 0.01);
            effect.update(false, 0.4);
            assert!((effect.current - 0.56).abs() < 1e-5, "native cubic exit");
            assert_eq!(effect.update(true, 0.01).vignette, 0.0, "native graph restart on re-entry");
            effect.update(true, 0.81);
            assert_eq!(effect.current, 1.0);
            effect.update(false, 0.01);
            effect.update(false, 0.81);
            assert_eq!(effect.current, 0.0);
        }
    }
}
