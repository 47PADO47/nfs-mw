//! Camera zoom uses real animation time. Spec: `speedbreaker-audio.md`.
#[derive(Default)]
pub(super) struct Zoom {
    blend: f64,
}

impl Zoom {
    pub fn update(&mut self, active: bool, real_dt: f32) {
        if !real_dt.is_finite() || real_dt <= 0.0 {
            return;
        }
        let (target, rate) = match active {
            true => (1.0, 4.0),
            false => (0.0, 1.0),
        };
        let step = f64::from(real_dt) * rate;
        self.blend += (target - self.blend).clamp(-step, step);
    }

    pub fn factor(&self) -> f32 {
        (1.0 - 0.4 * self.blend) as f32
    }
}

impl super::Drive {
    pub(in crate::scenes::world) fn speedbreaker_zoom(&self) -> f32 {
        self.breaker_camera.factor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_zoom_uses_real_time_including_frames_without_physics_steps() {
        for fps in [5, 30, 60, 120, 144, 240, 360] {
            let mut zoom = Zoom::default();
            for _ in 0..fps {
                zoom.update(true, 1.0 / fps as f32);
            }
            assert_eq!(zoom.factor(), 0.6, "held at {fps}");
            for _ in 0..fps {
                zoom.update(false, 1.0 / fps as f32);
            }
            assert!((zoom.factor() - 1.0).abs() < 1e-6, "released at {fps}");
        }
        let mut zoom = Zoom::default();
        zoom.update(true, 0.125);
        assert_eq!(zoom.factor(), 0.8);
        zoom.update(false, 0.25);
        assert_eq!(zoom.factor(), 0.9);
        zoom.update(true, 0.0625);
        assert_eq!(zoom.factor(), 0.8);
        zoom.update(true, 10.0);
        assert_eq!(zoom.factor(), 0.6);
        zoom.update(false, 0.0);
        zoom.update(false, f32::NAN);
        assert_eq!(zoom.factor(), 0.6, "pause/invalid deltas retain state");
    }
}
