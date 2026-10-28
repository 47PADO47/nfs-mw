//! Synthetic presentation history; no physics or gameplay events are generated.
use super::TailLightTrails;
use crate::settings::TailPreview;

impl TailLightTrails {
    pub fn preview_active(&self) -> bool {
        self.preview.is_some()
    }

    pub fn set_preview(&mut self, config: Option<TailPreview>) {
        let config = config.map(|c| TailPreview {
            speed_kmh: c.speed_kmh.clamp(10, 400),
            bend: c.bend.clamp(-100, 100),
            cycle: c.cycle,
        });
        if config == self.preview.map(|(c, _)| c) {
            return;
        }
        let started = self.preview.map_or_else(std::time::Instant::now, |(_, start)| start);
        self.smooth = Default::default();
        self.preview = config.map(|c| (c, started));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat4, Vec3};

    #[test]
    fn preview_is_anchored_curved_and_cleared_on_exit() {
        let mut trails = TailLightTrails::default();
        let pose = Mat4::from_translation(Vec3::new(10.0, 20.0, 3.0));
        let lamp = Vec3::new(-2.0, 0.5, 0.7);
        trails.set_preview(Some(TailPreview::default()));
        trails.advance_visual(0.0, pose, true);
        let straight = trails.path(lamp, 100, 100);
        assert!(straight.len() > 2);
        assert!(straight[0].abs_diff_eq(pose.transform_point3(lamp), 1e-5));
        assert!((trails.speed() - 240.0 / 3.6).abs() < 1e-4);
        for bend in [-100, 100] {
            trails.set_preview(Some(TailPreview { bend, ..Default::default() }));
            trails.advance_visual(0.0, pose, true);
            let curved = trails.path(lamp, 100, 100);
            assert!(curved.last().unwrap().distance(*straight.last().unwrap()) > 0.3);
            assert!(curved[0].abs_diff_eq(straight[0], 1e-5));
        }
        assert!(!trails.active, "preview must not activate real NOS history");
        trails.set_preview(None);
        assert!(!trails.preview_active());
        assert!(trails.path(lamp, 100, 100).is_empty());
        trails.advance_visual(1.0 / 60.0, pose, true);
        assert!(trails.path(lamp, 100, 100).is_empty());
    }
}
