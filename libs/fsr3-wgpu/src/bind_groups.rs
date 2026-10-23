//! Bind groups kept between frames. A steady upscale binds the same textures and buffers every frame, so the
//! groups are made once per variant (which of the swapped textures is current, which constants block is
//! used) and reused until the application hands in other views or the internal images are recreated.

use wgpu::{BindGroup, TextureView};

use crate::inputs::{Fsr3Inputs, Fsr3Outputs};

/// The views the cached groups were built from. A group binds these directly, so a changed view makes the
/// whole cache stale.
struct Views {
    color: TextureView,
    depth: TextureView,
    motion_vectors: TextureView,
    exposure: Option<TextureView>,
    reactive: Option<TextureView>,
    transparency: Option<TextureView>,
    output: TextureView,
}

impl Views {
    fn new(inputs: &Fsr3Inputs<'_>, outputs: &Fsr3Outputs<'_>) -> Self {
        Self {
            color: inputs.color.clone(),
            depth: inputs.depth.clone(),
            motion_vectors: inputs.motion_vectors.clone(),
            exposure: inputs.exposure.cloned(),
            reactive: inputs.reactive.cloned(),
            transparency: inputs.transparency_and_composition.cloned(),
            output: outputs.output.clone(),
        }
    }

    fn matches(&self, inputs: &Fsr3Inputs<'_>, outputs: &Fsr3Outputs<'_>) -> bool {
        self.color == *inputs.color
            && self.depth == *inputs.depth
            && self.motion_vectors == *inputs.motion_vectors
            && self.exposure.as_ref() == inputs.exposure
            && self.reactive.as_ref() == inputs.reactive
            && self.transparency.as_ref() == inputs.transparency_and_composition
            && self.output == *outputs.output
    }
}

/// One slot per pass variant; a slot is filled on first use.
#[derive(Default)]
pub(crate) struct BindGroups {
    views: Option<Views>,
    slots: Vec<Option<BindGroup>>,
}

impl BindGroups {
    /// Drops every group, for when the internal images were recreated.
    pub fn clear(&mut self) {
        self.views = None;
        self.slots.clear();
    }

    /// Drops the groups unless they were built from exactly these views.
    pub fn sync(&mut self, inputs: &Fsr3Inputs<'_>, outputs: &Fsr3Outputs<'_>) {
        if self.views.as_ref().is_some_and(|views| views.matches(inputs, outputs)) {
            return;
        }
        self.clear();
        self.views = Some(Views::new(inputs, outputs));
    }

    /// The group in `slot`; `make` runs only the first time the slot is asked for.
    pub fn get_or_insert_with(&mut self, slot: usize, make: impl FnOnce() -> BindGroup) -> &BindGroup {
        if self.slots.len() <= slot {
            self.slots.resize_with(slot + 1, || None);
        }
        self.slots[slot].get_or_insert_with(make)
    }
}
