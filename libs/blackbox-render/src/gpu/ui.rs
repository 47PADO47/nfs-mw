//! The UI layer on the renderer: `blackbox-gpu-passes` draws it, this wires it to the device and the output.

use super::Renderer;
use crate::{UiLayer, UiTextureId, UiTexturePatch};

pub(super) use blackbox_gpu_passes::UiPass as Ui;

impl Renderer {
    /// Create, replace or update a UI texture. A bad patch (wrong byte count, a region outside the
    /// texture, or a region of a texture that does not exist) is logged and ignored.
    pub fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>) {
        if let Err(e) = self.ui.update_texture(&self.device, &self.queue, patch) {
            log::warn!("{e}");
        }
    }

    /// Free a UI texture. Meshes that still name it are skipped.
    pub fn free_ui_texture(&mut self, id: UiTextureId) {
        self.ui.free_texture(id);
    }

    /// The UI to draw over every following frame, until replaced. An empty layer draws nothing.
    pub fn set_ui_layer(&mut self, layer: UiLayer) {
        self.ui.set_layer(layer);
    }

    /// Draw the UI layer into `target`, which is `size` pixels in the output format, on top of what is
    /// already there.
    pub(super) fn encode_ui(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: (u32, u32),
    ) {
        let format = self.output.format();
        self.ui.encode(&self.device, &self.queue, encoder, target, format, size);
    }
}
