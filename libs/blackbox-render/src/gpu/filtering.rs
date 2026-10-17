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
        for (slot, view) in &self.texture_views {
            let Some(group) = self.textures.get_mut(*slot) else { continue };
            *group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("scene texture filtering"),
                layout: &self.shared.bindings.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });
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
