//! The clip-plane variants of the scene pipelines.
//!
//! A planar-reflection capture discards everything behind its mirror plane in the fragment shaders. Compiling that
//! `discard` into the main view's pipelines would turn off early depth testing for every opaque draw, so the
//! shaders take it as a pipeline-overridable constant and only the capture's pipelines set it.

use super::Renderer;

impl Renderer {
    /// Draw the next scene pass with the clip-plane variants of the scene, glossy, normal and window pipelines
    /// (`clip`) or the plain ones. Called at the start of every scene pass, before the pipelines are prepared.
    pub(super) fn select_clip_variant(&mut self, clip: bool) {
        self.pipelines.use_clip(clip);
        if let Some(normal) = &mut self.normal {
            normal.use_clip(clip);
        }
        if let Some(window) = &mut self.window {
            window.use_clip(clip);
        }
    }
}
