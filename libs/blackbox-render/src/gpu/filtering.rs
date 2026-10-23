//! Live surface-sampler replacement; textures, meshes and material handles remain stable.
use super::Renderer;
use blackbox_gfx::TextureFiltering;

impl Renderer {
    pub fn set_texture_filtering(&mut self, requested: TextureFiltering) {
        let filtering = requested.supported(self.caps.max_texture_anisotropy);
        if self.graphics.texture_filtering == filtering {
            return;
        }
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scene surface filtering"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: filtering.anisotropy(),
            ..Default::default()
        });
        self.shared.bindings.sampler = sampler.clone();
        let groups: Vec<_> = self
            .texture_views
            .iter()
            .map(|(&slot, view)| {
                let projected = self
                    .projected
                    .as_ref()
                    .filter(|_| self.planar_receivers.contains(&slot))
                    .map(|capture| (capture.output(), capture.receiver_matrix));
                (slot, self.diffuse_group(view, projected))
            })
            .collect();
        for (slot, group) in groups {
            if let Some(old) = self.textures.get_mut(slot) {
                *old = group;
            }
        }
        if let Some(normal) = &mut self.normal {
            normal.set_sampler(&self.device, &sampler);
        }
        if let Some(window) = &mut self.window {
            window.set_sampler(&self.device, &sampler);
        }
        if let Some(sky) = &mut self.sky {
            sky.set_sampler(&self.device, &sampler);
        }
        self.shared.bindings.sampler = sampler;
        self.graphics.texture_filtering = filtering;
    }
}

#[cfg(test)]
#[path = "filtering_tests.rs"]
mod tests;
