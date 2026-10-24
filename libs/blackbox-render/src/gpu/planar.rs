//! Diffuse receivers keep their normal draw bucket and authored alpha/order.
use super::Renderer;
use crate::TextureHandle;
use glam::Mat4;
use wgpu::util::DeviceExt;

pub(super) struct Bindings {
    fallback: wgpu::TextureView,
    sampler: wgpu::Sampler,
    disabled: wgpu::Buffer,
}

pub(super) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let texture = wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    };
    let sampler = wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering);
    let uniform = wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("diffuse and planar receiver"),
        entries: &std::array::from_fn::<_, 5, _>(|binding| wgpu::BindGroupLayoutEntry {
            binding: binding as u32,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: match binding {
                0 | 2 => texture,
                1 | 3 => sampler,
                _ => uniform,
            },
            count: None,
        }),
    })
}

impl Bindings {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let fallback = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("empty planar image"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("planar clamp"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let disabled = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("disabled planar receiver"),
            contents: &[0; 80],
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Self { fallback, sampler, disabled }
    }
}

impl Renderer {
    pub(super) fn diffuse_group(
        &self,
        diffuse: &wgpu::TextureView,
        projected: Option<(&wgpu::TextureView, Mat4)>,
    ) -> wgpu::BindGroup {
        let (image, matrix, enabled) = match projected {
            Some((image, matrix)) => (image, matrix, 1.0),
            None => (&self.shared.planar.fallback, Mat4::ZERO, 0.0),
        };
        let params = projected.map(|_| {
            let mut words = matrix.to_cols_array().to_vec();
            words.extend([enabled, 0.0, 0.0, 0.0]);
            self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("planar receiver projection"),
                contents: bytemuck::cast_slice(&words),
                usage: wgpu::BufferUsages::UNIFORM,
            })
        });
        let params = params.as_ref().unwrap_or(&self.shared.planar.disabled);
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse and planar receiver"),
            layout: &self.shared.bindings.texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(diffuse) },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.shared.bindings.sampler),
                },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(image) },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.shared.planar.sampler),
                },
                wgpu::BindGroupEntry { binding: 4, resource: params.as_entire_binding() },
            ],
        })
    }

    pub(super) fn clear_planar_receivers(&mut self) {
        for handle in std::mem::take(&mut self.planar_receivers) {
            let Some(view) = self.texture_views.get(&handle) else { continue };
            let group = self.diffuse_group(view, None);
            if let Some(slot) = self.textures.get_mut(handle) {
                *slot = group;
            }
        }
    }

    pub(super) fn bind_planar_receivers(
        &mut self,
        receivers: &[TextureHandle],
        matrix: Mat4,
        image: &wgpu::TextureView,
    ) {
        self.clear_planar_receivers();
        for handle in receivers {
            let Some(view) = self.texture_views.get(&handle.raw()) else { continue };
            let group = self.diffuse_group(view, Some((image, matrix)));
            if let Some(slot) = self.textures.get_mut(handle.raw()) {
                *slot = group;
            }
            self.planar_receivers.push(handle.raw());
        }
    }
}
