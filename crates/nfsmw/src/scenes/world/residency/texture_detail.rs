use super::Residency;

impl Residency {
    /// Capture once: a running scene never mixes texture qualities while streaming.
    pub(in crate::scenes::world) fn capture_texture_detail(&mut self, discard: u32) {
        self.texture_discard.get_or_insert(discard);
    }
}
