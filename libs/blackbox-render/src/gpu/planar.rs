//! Diffuse receivers keep their normal draw bucket and authored alpha/order.
use super::Renderer;
use crate::TextureHandle;
use blackbox_gpu_passes::BindCache;
use glam::Mat4;
use wgpu::util::DeviceExt;

pub(super) struct Bindings {
    fallback: wgpu::TextureView,
    sampler: wgpu::Sampler,
    disabled: wgpu::Buffer,
    /// The projection every receiver of the current planar capture samples with. Rewritten, never recreated.
    projection: wgpu::Buffer,
    scroll_zero: wgpu::Buffer,
    /// Receiver groups with the planar image off, keyed by the receiver's own view.
    defaults: BindCache<wgpu::TextureView, wgpu::BindGroup>,
    /// Receiver groups sampling a planar image, keyed by the receiver's view and that image.
    enabled: BindCache<wgpu::TextureView, wgpu::BindGroup>,
    /// The surface sampler the cached groups were built with.
    built_with: Option<wgpu::Sampler>,
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
        entries: &std::array::from_fn::<_, 6, _>(|binding| wgpu::BindGroupLayoutEntry {
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
        let projection = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("planar receiver projection"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scroll_zero = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("zero texture scroll"),
            contents: &[0; 16],
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Self {
            fallback,
            sampler,
            disabled,
            projection,
            scroll_zero,
            defaults: BindCache::default(),
            enabled: BindCache::default(),
            built_with: None,
        }
    }

    pub(super) fn write_projection(&self, queue: &wgpu::Queue, matrix: Mat4) {
        queue.write_buffer(&self.projection, 0, bytemuck::cast_slice(&projection_words(matrix)));
    }

    /// Forget the cached groups when the surface sampler they were built with has been replaced.
    fn use_sampler(&mut self, sampler: &wgpu::Sampler) {
        if self.built_with.as_ref() == Some(sampler) {
            return;
        }
        self.defaults = BindCache::default();
        self.enabled = BindCache::default();
        self.built_with = Some(sampler.clone());
    }

    /// The group of the receiver `diffuse`: sampling `image` with the current projection, or with the planar
    /// image off. Made once per input pair, then reused every frame.
    fn receiver_group(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        diffuse: &wgpu::TextureView,
        image: Option<&wgpu::TextureView>,
        scroll: Option<&wgpu::Buffer>,
    ) -> wgpu::BindGroup {
        if let Some(scroll) = scroll {
            return match image {
                Some(image) => {
                    build_group(device, layout, sampler, &self.sampler, diffuse, image, &self.projection, scroll)
                }
                None => {
                    build_group(device, layout, sampler, &self.sampler, diffuse, &self.fallback, &self.disabled, scroll)
                }
            };
        }
        let Some(image) = image else {
            let make = || {
                build_group(
                    device,
                    layout,
                    sampler,
                    &self.sampler,
                    diffuse,
                    &self.fallback,
                    &self.disabled,
                    &self.scroll_zero,
                )
            };
            return self.defaults.get_or_insert_with(diffuse, &self.fallback, make).clone();
        };
        let make =
            || build_group(device, layout, sampler, &self.sampler, diffuse, image, &self.projection, &self.scroll_zero);
        self.enabled.get_or_insert_with(diffuse, image, make).clone()
    }
}

/// The uniform of a receiver: the projection, then the enabled flag.
fn projection_words(matrix: Mat4) -> [f32; 20] {
    let mut words = [0.0; 20];
    words[..16].copy_from_slice(&matrix.to_cols_array());
    words[16] = 1.0;
    words
}

fn build_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    planar_sampler: &wgpu::Sampler,
    diffuse: &wgpu::TextureView,
    image: &wgpu::TextureView,
    params: &wgpu::Buffer,
    scroll: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("diffuse and planar receiver"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(diffuse) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(image) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(planar_sampler) },
            wgpu::BindGroupEntry { binding: 4, resource: params.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: scroll.as_entire_binding() },
        ],
    })
}

impl Renderer {
    /// An uncached group, for a texture that was just made or whose sampler just changed.
    pub(super) fn diffuse_group(
        &self,
        diffuse: &wgpu::TextureView,
        projected: Option<(&wgpu::TextureView, Mat4)>,
        scroll: Option<&wgpu::Buffer>,
    ) -> wgpu::BindGroup {
        let planar = &self.shared.planar;
        let (image, params) = match projected {
            Some((image, matrix)) => {
                planar.write_projection(&self.queue, matrix);
                (image, &planar.projection)
            }
            None => (&planar.fallback, &planar.disabled),
        };
        let bindings = &self.shared.bindings;
        build_group(
            &self.device,
            &bindings.texture_layout,
            &bindings.sampler,
            &planar.sampler,
            diffuse,
            image,
            params,
            scroll.unwrap_or(&planar.scroll_zero),
        )
    }

    pub(super) fn clear_planar_receivers(&mut self) {
        if self.planar_receivers.is_empty() {
            return;
        }
        self.shared.planar.use_sampler(&self.shared.bindings.sampler);
        for handle in std::mem::take(&mut self.planar_receivers) {
            let Some(view) = self.texture_views.get(&handle) else { continue };
            let bindings = &self.shared.bindings;
            let layout = &bindings.texture_layout;
            let group = self.shared.planar.receiver_group(
                &self.device,
                layout,
                &bindings.sampler,
                view,
                None,
                self.texture_scrolls.get(&handle),
            );
            if let Some(slot) = self.textures.get_mut(handle) {
                *slot = group;
            }
        }
    }

    /// Point `receivers` at `image`. The previous receivers were already cleared by the capture.
    pub(super) fn bind_planar_receivers(
        &mut self,
        receivers: &[TextureHandle],
        matrix: Mat4,
        image: &wgpu::TextureView,
    ) {
        self.shared.planar.write_projection(&self.queue, matrix);
        self.shared.planar.use_sampler(&self.shared.bindings.sampler);
        for handle in receivers {
            let Some(view) = self.texture_views.get(&handle.raw()) else { continue };
            let bindings = &self.shared.bindings;
            let group = self.shared.planar.receiver_group(
                &self.device,
                &bindings.texture_layout,
                &bindings.sampler,
                view,
                Some(image),
                self.texture_scrolls.get(&handle.raw()),
            );
            if let Some(slot) = self.textures.get_mut(handle.raw()) {
                *slot = group;
            }
            self.planar_receivers.push(handle.raw());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uniform_is_the_matrix_then_the_enabled_flag() {
        let matrix = Mat4::from_translation(glam::Vec3::new(1.0, 2.0, 3.0));
        let words = projection_words(matrix);
        assert_eq!(words[..16], matrix.to_cols_array());
        assert_eq!(words[16..], [1.0, 0.0, 0.0, 0.0]);
    }
}
