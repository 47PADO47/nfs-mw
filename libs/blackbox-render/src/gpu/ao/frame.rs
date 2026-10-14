//! The ambient visibility texture is ready before the scene's colour draw.
use super::{AmbientOcclusion, Draw};
use crate::gpu::Renderer;
use blackbox_gfx::FrameParams;

impl Renderer {
    pub(in crate::gpu) fn encode_ambient(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &FrameParams) {
        if !self.graphics.ao.active() || !self.screen_lighting_active(frame) {
            if self.shared.shadows.ambient.texture().width() != 1 {
                self.shared.shadows.set_ambient(
                    &self.device,
                    &self.shared.glossy.material,
                    super::super::surface::mask(&self.device, (1, 1), "disabled ambient mask"),
                );
                if let Some(glossy) = &mut self.glossy {
                    glossy.rebind_shadows(&self.device, &self.shared.shadows);
                }
            }
            self.ao = None;
            return;
        }
        let Some(surfaces) = &self.surfaces else { return };
        let targets = &surfaces.targets;
        let ao = self.ao.get_or_insert_with(|| AmbientOcclusion::new(&self.device, targets.size));
        if ao.size != targets.size {
            ao.size = targets.size;
            ao.mask = super::super::surface::mask(&self.device, targets.size, "ambient visibility");
        }
        if self.shared.shadows.ambient != ao.mask {
            self.shared.shadows.set_ambient(&self.device, &self.shared.glossy.material, ao.mask.clone());
            if let Some(glossy) = &mut self.glossy {
                glossy.rebind_shadows(&self.device, &self.shared.shadows);
            }
        }
        let mask = ao.mask.clone();
        ao.encode(
            &self.device,
            &self.queue,
            encoder,
            Draw {
                target: &mask,
                depth: &targets.depth,
                normals: &targets.normals,
                size: targets.size,
                frame,
                settings: self.graphics.ao,
            },
        );
    }
}
