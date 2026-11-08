//! The colour grading of the world, handed to the renderer every frame (`docs/specs/colour-grading.md`).
//!
//! The scene keeps the look of the grading variant the player chose (`scene_set_effect_variants`), and each frame picks the grade
//! for the clock's hour and how wet the weather is (`grading::Selector`), at the strength setting. A renderer without the
//! pass (`Capabilities::colour_cube`) is not asked. Nothing is sent while there is no look, so a game without a grading mod draws
//! exactly what it did.

use std::sync::Arc;

use blackbox_gfx::RenderBackend;

use super::WorldScene;
use crate::grading::{GradingLook, Selector, wet_factor};

/// What the scene keeps for the grading.
#[derive(Default)]
pub(super) struct GradingFeed {
    look: Option<Arc<GradingLook>>,
    selector: Selector,
    /// A grade was given to the renderer and not taken back.
    sent: bool,
}

impl GradingFeed {
    pub(super) fn set_look(&mut self, look: Option<Arc<GradingLook>>) {
        self.look = look;
    }
}

impl WorldScene {
    /// Give the renderer this frame's grade, or take the last one back when there is none.
    pub(super) fn feed_grading(&mut self, renderer: &mut dyn RenderBackend) {
        if !renderer.capabilities().colour_cube {
            return;
        }
        // The garage's backdrop has its own look.
        let look = self.grading.look.clone().filter(|_| self.garage_backdrop.is_none());
        let grade = look.and_then(|look| {
            let wet = wet_factor(self.rain.intensity(), self.wet_road.amount());
            self.grading.selector.grade(&look, self.day.hour(), wet, self.tuning.grading_strength)
        });
        if grade.is_none() && !self.grading.sent {
            return;
        }
        self.grading.sent = grade.is_some();
        renderer.set_colour_grade(grade.as_ref());
    }
}
