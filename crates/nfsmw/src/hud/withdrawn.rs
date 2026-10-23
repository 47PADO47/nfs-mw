//! Taking the HUD off the screen. The render bridge keeps the last UI layer it was handed until it gets a new one, and
//! the presenter hands one over only while the HUD is drawn: a HUD that stops being drawn (a cutscene starts, the HUD
//! setting goes off) would stay on screen as it was on its last frame. The first hidden frame hands over a layer without
//! it instead; the frames after it hand over nothing, so a layer another pass made is left alone.

use crate::gui::UiOutput;
use blackbox_gfx::UiLayer;

/// Whether the HUD was drawn on the last frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Drawn(bool);

impl Drawn {
    /// A frame the HUD is drawn on.
    pub fn shown(&mut self) {
        self.0 = true;
    }

    /// A frame the HUD is not drawn on: the first one hands the renderer a layer without it.
    pub fn hidden(&mut self, out: &mut UiOutput, pixels_per_point: f32) {
        if !self.0 {
            return;
        }
        self.0 = false;
        out.layer.get_or_insert_with(|| UiLayer { pixels_per_point, ..Default::default() });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{UiMesh, UiTextureId};

    #[test]
    fn the_first_hidden_frame_clears_the_layer_once() {
        let mut drawn = Drawn::default();
        let mut out = UiOutput::default();
        drawn.hidden(&mut out, 1.0);
        assert!(out.layer.is_none(), "never drawn: nothing to take off");
        drawn.shown();
        drawn.hidden(&mut out, 2.0);
        let layer = out.layer.take().expect("an empty layer replaces the HUD");
        assert!(layer.meshes.is_empty());
        assert_eq!(layer.pixels_per_point, 2.0);
        drawn.hidden(&mut out, 2.0);
        assert!(out.layer.is_none(), "only the first hidden frame hands one over");
    }

    #[test]
    fn a_layer_another_pass_made_is_kept() {
        let mut drawn = Drawn::default();
        drawn.shown();
        let mesh = UiMesh { vertices: Vec::new(), indices: Vec::new(), texture: UiTextureId(7), clip: [0.0; 4] };
        let mut out =
            UiOutput { layer: Some(UiLayer { meshes: vec![mesh], ..Default::default() }), ..Default::default() };
        drawn.hidden(&mut out, 1.0);
        assert_eq!(out.layer.map(|l| l.meshes.len()), Some(1));
    }
}
