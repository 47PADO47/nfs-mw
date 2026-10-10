//! Remembers that the next frame is a camera cut (a teleport, a camera switch, a new scene), so the
//! frame says so to a renderer that reuses the last frame.

/// Armed from the start: a scene's first frame is always a cut.
#[derive(Debug, Clone, Copy)]
pub struct CameraCut(bool);

impl Default for CameraCut {
    fn default() -> Self {
        Self(true)
    }
}

impl CameraCut {
    /// The camera jumped: the next frame is a cut.
    pub fn arm(&mut self) {
        self.0 = true;
    }

    /// Whether the frame being built is a cut; the answer is yes once per [`arm`](Self::arm).
    pub fn take(&mut self) -> bool {
        std::mem::take(&mut self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_frame_is_a_cut_and_then_only_when_armed() {
        let mut cut = CameraCut::default();
        assert!(cut.take());
        assert!(!cut.take());
        cut.arm();
        assert!(cut.take());
        assert!(!cut.take());
    }
}
