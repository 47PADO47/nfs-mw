//! The frame a paused or concealed world presents.
use blackbox_gfx::{FrameParams, Instance};

use super::{Pausable, flat_frame};

impl Pausable {
    pub(super) fn scene_frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        if !self.concealed() {
            return self.inner.frame(aspect);
        }
        // The scene still takes its frame (its camera and physics read the aspect ratio), but nothing is drawn.
        let _ = self.inner.frame(aspect);
        (flat_frame(), &[] as &[Instance])
    }

    /// A frozen or concealed world does not move on its own: the camera and the instances are what the picture is. A
    /// pending refresh of its effect buffers changes renderer state, so that frame is not still.
    pub(super) fn still_frame(&self) -> bool {
        if self.effects_dirty {
            return false;
        }
        if self.concealed() || (self.paused.get() && !self.inner.runs_while_paused()) {
            return true;
        }
        self.inner.menu_frame()
    }
}
