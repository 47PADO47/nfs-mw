//! Setting the UI layer from a borrowed one, reusing the copy the pass already holds.
use super::UiPass;
use blackbox_gfx::UiLayer;

impl UiPass {
    /// [`Self::set_layer`] for a caller that keeps its layer: the meshes are copied into the layer the pass holds,
    /// reusing its allocations, and only when they differ. The compositions of `layer` are queued as usual.
    pub fn set_layer_ref(&mut self, layer: &UiLayer) {
        for command in &layer.compositions {
            self.pending_compositions.insert(command.output, *command);
        }
        if self.layer.pixels_per_point == layer.pixels_per_point && self.layer.meshes == layer.meshes {
            return;
        }
        self.layer.assign_meshes(layer);
        self.stale = true;
    }
}
