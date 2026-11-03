use super::*;
impl Normal {
    pub(in crate::gpu) fn set_sampler(&mut self, device: &wgpu::Device, sampler: &wgpu::Sampler) {
        self.sampler = sampler.clone();
        for material in self.materials.iter_mut() {
            material.bind_group = material_group(
                device,
                &self.material_layout,
                sampler,
                &self.lighting,
                &material.views,
                &material.offset,
                &self.water,
                false,
            );
            material.water_group = None;
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn material_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    lighting: &wgpu::Buffer,
    views: &[wgpu::TextureView; 4],
    offset: &wgpu::Buffer,
    water: &water::Water,
    active: bool,
) -> wgpu::BindGroup {
    let entries = vec![
        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&views[0]) },
        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&views[1]) },
        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&views[2]) },
        wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(sampler) },
        wgpu::BindGroupEntry { binding: 4, resource: offset.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 5, resource: lighting.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&views[3]) },
        wgpu::BindGroupEntry {
            binding: 7,
            resource: match active {
                true => water.screen.control.as_entire_binding(),
                false => water.disabled.as_entire_binding(),
            },
        },
        wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&water.screen.color) },
        wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::TextureView(&water.screen.depth) },
        wgpu::BindGroupEntry {
            binding: 10,
            resource: wgpu::BindingResource::TextureView(match active {
                true => &water.environment,
                false => &water.base_environment,
            }),
        },
        wgpu::BindGroupEntry { binding: 11, resource: wgpu::BindingResource::Sampler(&water.sampler) },
        wgpu::BindGroupEntry {
            binding: 12,
            resource: wgpu::BindingResource::TextureView(match active {
                true => &water.filtered_environment,
                false => &water.base_environment,
            }),
        },
        wgpu::BindGroupEntry {
            binding: 13,
            resource: wgpu::BindingResource::TextureView(match active {
                true => &water.probe_depth,
                false => &water.base_probe_depth,
            }),
        },
        wgpu::BindGroupEntry {
            binding: 14,
            resource: match active {
                true => water.probe_origin.as_entire_binding(),
                false => water.disabled.as_entire_binding(),
            },
        },
        wgpu::BindGroupEntry { binding: 15, resource: water.sky.as_entire_binding() },
    ];
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("normal scenery material"),
        layout,
        entries: &entries,
    })
}

impl Normal {
    pub(in crate::gpu) fn set_projected(&mut self, device: &wgpu::Device, view: Option<&wgpu::TextureView>) {
        let same = match (&self.projected, view) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.projected = view.cloned();
        let view = self.projected.as_ref().unwrap_or(&self.black_reflection);
        for material in self.materials.iter_mut().filter(|m| m.uses_projected) {
            material.views[3] = view.clone();
            material.bind_group = material_group(
                device,
                &self.material_layout,
                &self.sampler,
                &self.lighting,
                &material.views,
                &material.offset,
                &self.water,
                false,
            );
            material.water_group = None;
        }
    }
    pub(in crate::gpu) fn hit_surface(&self, handle: NormalMaterialHandle) -> Option<NormalMaterialDesc> {
        self.materials.get(handle.raw()).map(|m| m.hit_surface)
    }
    pub(in crate::gpu) fn capture_diffuse(&self, handle: NormalMaterialHandle) -> Option<blackbox_gfx::TextureHandle> {
        self.materials.get(handle.raw()).map(|m| m.diffuse)
    }
}
