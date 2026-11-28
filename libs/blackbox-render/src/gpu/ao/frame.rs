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
        let jittered = self.jittered();
        let Some(surfaces) = &self.surfaces else { return };
        let targets = &surfaces.targets;
        let ao = self
            .ao
            .get_or_insert_with(|| AmbientOcclusion::new(&self.device, targets.size, self.pipeline_cache.as_ref()));
        let format = match self.graphics.ao.directional() {
            true => wgpu::TextureFormat::Rgba8Unorm,
            false => wgpu::TextureFormat::R8Unorm,
        };
        if ao.size != targets.size || ao.mask.texture().format() != format {
            ao.size = targets.size;
            ao.mask = super::super::surface::mask_with_format(&self.device, targets.size, "ambient visibility", format);
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
                motion: self.effect_motion.as_ref(),
                target: &mask,
                depth: &targets.depth,
                normals: &targets.normals,
                static_depth: surfaces.static_depth.as_ref().and_then(|s| s.current()),
                size: targets.size,
                frame,
                settings: self.graphics.ao,
                jittered,
            },
        );
    }
}
