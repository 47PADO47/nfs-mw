//! Shared receiver bindings. Disabled shadows use a tiny placeholder; quality sizes the real atlas.
use super::fit::Params;
use wgpu::util::DeviceExt;

pub(in crate::gpu) struct Maps {
    pub resolution: u32,
    pub detail_resolution: Option<u32>,
    pub layer_count: u32,
    pub buffer: wgpu::Buffer,
    pub view: wgpu::TextureView,
    pub(in crate::gpu) detail: wgpu::TextureView,
    pub layers: Vec<wgpu::TextureView>,
    pub sampler: wgpu::Sampler,
    pub fallback: wgpu::BindGroup,
    pub contact: wgpu::TextureView,
    pub ambient: wgpu::TextureView,
    pub cloud: wgpu::TextureView,
    dummy: wgpu::Buffer,
}

impl Maps {
    pub(in crate::gpu) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        resolution: u32,
        layer_count: u32,
        detail_resolution: Option<u32>,
        contact: Option<wgpu::TextureView>,
        ambient: Option<wgpu::TextureView>,
    ) -> Self {
        let contact = contact.unwrap_or_else(|| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("disabled contact mask"),
                    size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        });
        let ambient = ambient.unwrap_or_else(|| crate::gpu::surface::mask(device, (1, 1), "disabled ambient mask"));
        let cloud = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("cloud sunlight transmission"),
                size: wgpu::Extent3d { width: 256, height: 256, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let (view, detail, layers) = match detail_resolution {
            Some(size) => {
                let (detail, mut layers) = atlas(device, size, 2);
                let (view, distant) = atlas(device, resolution, layer_count - 2);
                layers.extend(distant);
                (view, detail, layers)
            }
            None => {
                let (view, layers) = atlas(device, resolution, layer_count);
                (view.clone(), view, layers)
            }
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow receiver data"),
            contents: bytemuck::bytes_of(&Params::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("spatial shadow comparison"),
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let dummy = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("unused scene material"),
            contents: &[0; 80],
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let fallback = group(device, layout, &dummy, &buffer, (&view, &detail), &sampler, (&contact, &ambient, &cloud));
        Self {
            resolution,
            detail_resolution,
            layer_count,
            buffer,
            view,
            detail,
            layers,
            sampler,
            fallback,
            contact,
            ambient,
            cloud,
            dummy,
        }
    }

    pub(in crate::gpu) fn material_group(
        &self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        material: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        group(
            device,
            layout,
            material,
            &self.buffer,
            (&self.view, &self.detail),
            &self.sampler,
            (&self.contact, &self.ambient, &self.cloud),
        )
    }
    pub(in crate::gpu) fn set_ambient(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        ambient: wgpu::TextureView,
    ) {
        self.ambient = ambient;
        self.fallback = self.material_group(device, layout, &self.dummy);
    }
    pub(in crate::gpu) fn set_contact(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        contact: wgpu::TextureView,
    ) {
        self.contact = contact;
        self.fallback = self.material_group(device, layout, &self.dummy);
    }
}

fn group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    material: &wgpu::Buffer,
    buffer: &wgpu::Buffer,
    views: (&wgpu::TextureView, &wgpu::TextureView),
    sampler: &wgpu::Sampler,
    masks: (&wgpu::TextureView, &wgpu::TextureView, &wgpu::TextureView),
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("material and shadow receivers"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: material.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(views.0) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(masks.0) },
            wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(masks.1) },
            wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(views.1) },
            wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(masks.2) },
        ],
    })
}

use bytemuck::Zeroable;

fn atlas(device: &wgpu::Device, resolution: u32, count: u32) -> (wgpu::TextureView, Vec<wgpu::TextureView>) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sun shadow maps"),
        size: wgpu::Extent3d { width: resolution, height: resolution, depth_or_array_layers: count },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let layers = (0..count)
        .map(|i| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: i,
                array_layer_count: Some(1),
                ..Default::default()
            })
        })
        .collect();
    (view, layers)
}
