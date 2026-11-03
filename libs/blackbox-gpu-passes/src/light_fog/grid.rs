//! Screen-tile light lists and a shared camera-aligned scattering volume.
use crate::WorldBindings;

/// Workgroup edge of the volume shaders.
const WORKGROUP: u32 = 8;
/// Fewest tiles along either screen axis, so tiny views keep some angular detail.
const MIN_TILES: u32 = 16;

pub(super) struct Grid {
    layout: wgpu::BindGroupLayout,
    tiles: wgpu::ComputePipeline,
    volume: wgpu::ComputePipeline,
    integrate: wgpu::ComputePipeline,
    shadow_layout: wgpu::BindGroupLayout,
    fallback_data: wgpu::Buffer,
    fallback_map: wgpu::TextureView,
    fallback_sampler: wgpu::Sampler,
    /// The shadow bind group of the last draw, kept while its inputs stay the same.
    shadow_group: Option<(ShadowKey, wgpu::BindGroup)>,
}

type ShadowKey = (wgpu::Buffer, wgpu::TextureView, wgpu::Sampler);

/// One view's cells: two history-swapped scattering volumes, the integrated volume and the tile masks.
pub(super) struct Volume {
    size: [u32; 3],
    cells: [wgpu::Buffer; 2],
    integrated: wgpu::Buffer,
    masks: wgpu::Buffer,
    /// Volume bind groups by the cell buffer written this frame.
    groups: [wgpu::BindGroup; 2],
    next: usize,
}

impl Grid {
    /// Whether `device` can run the volume: compute shaders with 8x8 workgroups and four storage buffers in a
    /// stage. Downlevel devices (WebGL 2, GL without compute) cannot, and draw no light fog.
    pub(super) fn supported(device: &wgpu::Device) -> bool {
        let limits = device.limits();
        limits.max_storage_buffers_per_shader_stage >= 4
            && limits.max_compute_workgroup_size_x >= WORKGROUP
            && limits.max_compute_workgroup_size_y >= WORKGROUP
            && limits.max_compute_invocations_per_workgroup >= WORKGROUP * WORKGROUP
            && limits.max_compute_workgroups_per_dimension > 0
    }

    pub(super) fn new(device: &wgpu::Device, bindings: &WorldBindings, cache: Option<&wgpu::PipelineCache>) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog volume inputs"),
            entries: &[
                buffer_entry(0, wgpu::BufferBindingType::Uniform),
                buffer_entry(1, wgpu::BufferBindingType::Storage { read_only: false }),
                buffer_entry(2, wgpu::BufferBindingType::Storage { read_only: false }),
                buffer_entry(3, wgpu::BufferBindingType::Storage { read_only: true }),
                buffer_entry(4, wgpu::BufferBindingType::Storage { read_only: false }),
            ],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fog shadow inputs"),
            entries: &[
                buffer_entry(0, wgpu::BufferBindingType::Uniform),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fog volume"),
            bind_group_layouts: &[Some(&bindings.globals_layout), Some(&layout), Some(&shadow_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fog volume"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/light_fog_grid.wgsl").into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache,
            })
        };
        let fallback_data = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("disabled fog shadows"),
            size: 16384,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let fallback_map = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("disabled fog shadow map"),
                size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
        let fallback_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Self {
            layout,
            tiles: pipeline("tile_lights"),
            volume: pipeline("build_volume"),
            integrate: pipeline("integrate_volume"),
            shadow_layout,
            fallback_data,
            fallback_map,
            fallback_sampler,
            shadow_group: None,
        }
    }

    /// Tiles across, tiles down and depth slices for a `width` x `height` view at `quality` (1 to 3).
    pub(super) fn dimensions(width: u32, height: u32, quality: u32) -> [u32; 3] {
        let (tile, max_x, max_y, slices) = match quality {
            1 => (24, 120, 68, 64),
            3 => (12, 240, 135, 128),
            _ => (16, 160, 90, 96),
        };
        [width.div_ceil(tile).clamp(MIN_TILES, max_x), height.div_ceil(tile).clamp(MIN_TILES, max_y), slices]
    }

    /// The buffers of a `size` volume whose parameters are read from `uniform`.
    pub(super) fn volume(&self, device: &wgpu::Device, size: [u32; 3], uniform: &wgpu::Buffer) -> Volume {
        let buffer = |label, count: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: count * 16,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let tiles = u64::from(size[0]) * u64::from(size[1]);
        let cells = [
            buffer("fog volume cells A", tiles * u64::from(size[2])),
            buffer("fog volume cells B", tiles * u64::from(size[2])),
        ];
        let integrated = buffer("integrated fog volume", tiles * u64::from(size[2]));
        let masks = buffer("fog tile light masks", tiles);
        let groups = std::array::from_fn(|next| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fog volume inputs"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: cells[next].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: masks.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: cells[1 - next].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: integrated.as_entire_binding() },
                ],
            })
        });
        Volume { size, cells, integrated, masks, groups, next: 0 }
    }

    /// Record the tile lists, the volume (blended with the other cell buffer as history) and its integration.
    pub(super) fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        globals: &wgpu::BindGroup,
        volume: &mut Volume,
        shadows: Option<&crate::LocalFogShadows<'_>>,
    ) {
        let key = match shadows {
            Some(shadows) => (shadows.data.clone(), shadows.map.clone(), shadows.sampler.clone()),
            None => (self.fallback_data.clone(), self.fallback_map.clone(), self.fallback_sampler.clone()),
        };
        if self.shadow_group.as_ref().is_none_or(|(cached, _)| *cached != key) {
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("fog shadow inputs"),
                layout: &self.shadow_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: key.0.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&key.1) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&key.2) },
                ],
            });
            self.shadow_group = Some((key, group));
        }
        let shadow_group = &self.shadow_group.as_ref().unwrap().1;
        let [x, y, z] = volume.size;
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("build local light fog volume"),
            timestamp_writes: None,
        });
        pass.set_bind_group(0, globals, &[]);
        pass.set_bind_group(1, &volume.groups[volume.next], &[]);
        pass.set_bind_group(2, shadow_group, &[]);
        pass.set_pipeline(&self.tiles);
        pass.dispatch_workgroups(x.div_ceil(WORKGROUP), y.div_ceil(WORKGROUP), 1);
        pass.set_pipeline(&self.volume);
        pass.dispatch_workgroups(x.div_ceil(WORKGROUP), y.div_ceil(WORKGROUP), z);
        pass.set_pipeline(&self.integrate);
        pass.dispatch_workgroups(x.div_ceil(WORKGROUP), y.div_ceil(WORKGROUP), 1);
        drop(pass);
        volume.next = 1 - volume.next;
    }
}

impl Volume {
    pub(super) fn size(&self) -> [u32; 3] {
        self.size
    }

    pub(super) fn integrated(&self) -> &wgpu::Buffer {
        &self.integrated
    }

    /// Bytes of GPU memory the volume holds.
    pub(super) fn bytes(&self) -> u64 {
        self.cells.iter().chain([&self.integrated, &self.masks]).map(wgpu::Buffer::size).sum()
    }
}

fn buffer_entry(binding: u32, ty: wgpu::BufferBindingType) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer { ty, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    }
}

#[cfg(test)]
mod tests {
    use super::Grid;

    #[test]
    fn low_quality_follows_the_view_and_high_resolutions_keep_their_grid() {
        assert_eq!(Grid::dimensions(1920, 1080, 1), [80, 45, 64], "no inflated minimum at 1080p");
        assert_eq!(Grid::dimensions(3840, 2160, 2), [160, 90, 96], "quality 2 at 4K is unchanged");
        assert_eq!(Grid::dimensions(3840, 2160, 3), [240, 135, 128]);
        assert_eq!(Grid::dimensions(64, 32, 2), [16, 16, 96], "tiny views keep a small minimum");
    }
}
