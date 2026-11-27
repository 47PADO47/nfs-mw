//! The camera depth of the static scene alone, taken in the middle of the prepass: after the world and before the
//! vehicles and characters. Ambient occlusion reads it to find the static surfaces a moving model hides, so a parked
//! car does not erase the occlusion of a wall's base it stands in front of. A mip chain of 16-pixel tiles marks
//! where a dynamic model covers the static scene, so pixels and taps far from one skip the extra reads.
use blackbox_gfx::Shading;

/// Models drawn after the snapshot: vehicles (lit and glossy materials) and characters. Everything else is scenery.
pub(super) fn dynamic(shading: Shading) -> bool {
    matches!(shading, Shading::Lit | Shading::Character | Shading::Glossy(_))
}

/// Screen pixels per side of a tile at the finest tile level.
pub(in crate::gpu) const TILE: u32 = 16;

struct Images {
    size: (u32, u32),
    depth: wgpu::TextureView,
    /// All tile levels, for the AO pass.
    tiles: wgpu::TextureView,
    levels: Vec<wgpu::TextureView>,
}

impl Images {
    fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let make = |label, size: (u32, u32), mips| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: mips,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        // Power-of-two sides halve exactly down to one tile, so the coarsest level covers the whole image.
        let grid = (size.0.div_ceil(TILE).next_power_of_two(), size.1.div_ceil(TILE).next_power_of_two());
        let mips = u32::BITS - grid.0.max(grid.1).leading_zeros();
        let tiles = make("dynamic model tiles", grid, mips);
        let levels = (0..mips)
            .map(|level| {
                tiles.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: level,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        Self {
            size,
            depth: make("static scene depth", size, 1).create_view(&Default::default()),
            tiles: tiles.create_view(&Default::default()),
            levels,
        }
    }
}

pub(in crate::gpu) struct StaticDepth {
    snapshot: wgpu::ComputePipeline,
    cover: wgpu::ComputePipeline,
    reduce: wgpu::ComputePipeline,
    images: Option<Images>,
    /// Bind groups for the scene depth they were made with: snapshot, cover, then one reduction per coarser level.
    groups: Option<(wgpu::TextureView, Vec<wgpu::BindGroup>)>,
    /// A dynamic model was drawn over the snapshot this frame.
    valid: bool,
}

impl StaticDepth {
    pub(in crate::gpu) fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("static depth"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/static_depth.wgsl").into()),
        });
        let pipeline = |entry| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: None,
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache,
            })
        };
        Self {
            snapshot: pipeline("snapshot"),
            cover: pipeline("cover"),
            reduce: pipeline("reduce"),
            images: None,
            groups: None,
            valid: false,
        }
    }

    /// This frame's static depth (reverse-Z, as the scene depth) and its tile levels, when a dynamic model covers
    /// part of the scene.
    pub(in crate::gpu) fn current(&self) -> Option<(&wgpu::TextureView, &wgpu::TextureView)> {
        let images = self.images.as_ref().filter(|_| self.valid)?;
        Some((&images.depth, &images.tiles))
    }

    pub(super) fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Copy `depth` (`size` pixels) as it stands: the static scene, before the dynamic models.
    pub(in crate::gpu) fn capture(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        size: (u32, u32),
    ) {
        if self.images.as_ref().is_none_or(|images| images.size != size) {
            self.images = Some(Images::new(device, size));
            self.groups = None;
        }
        self.bind(device, depth);
        let groups = &self.groups.as_ref().expect("static depth groups exist").1;
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("static depth snapshot"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.snapshot);
        pass.set_bind_group(0, &groups[0], &[]);
        pass.dispatch_workgroups(size.0.div_ceil(8), size.1.div_ceil(8), 1);
        self.valid = false;
    }

    /// After the dynamic models are drawn into `depth`: mark the tiles they cover. Follows [`capture`](Self::capture).
    pub(in crate::gpu) fn mark(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
    ) {
        self.bind(device, depth);
        let images = self.images.as_ref().expect("captured before marking");
        let groups = &self.groups.as_ref().expect("static depth groups exist").1;
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("dynamic model tiles"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.cover);
        pass.set_bind_group(0, &groups[1], &[]);
        let grid = images.levels[0].texture().size();
        pass.dispatch_workgroups(grid.width, grid.height, 1);
        pass.set_pipeline(&self.reduce);
        for (level, group) in groups[2..].iter().enumerate() {
            let size =
                images.levels[level + 1].texture().size().mip_level_size(level as u32 + 1, wgpu::TextureDimension::D2);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(size.width.div_ceil(8), size.height.div_ceil(8), 1);
        }
        self.valid = true;
    }

    fn bind(&mut self, device: &wgpu::Device, depth: &wgpu::TextureView) {
        if self.groups.as_ref().is_some_and(|(stored, _)| stored == depth) {
            return;
        }
        let images = self.images.as_ref().expect("static depth images exist");
        let view = |binding, view| wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::TextureView(view) };
        let group = |pipeline: &wgpu::ComputePipeline, entries: &[wgpu::BindGroupEntry<'_>]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("static depth"),
                layout: &pipeline.get_bind_group_layout(0),
                entries,
            })
        };
        let mut groups = vec![
            group(&self.snapshot, &[view(0, depth), view(1, &images.depth)]),
            group(&self.cover, &[view(0, depth), view(2, &images.depth), view(3, &images.levels[0])]),
        ];
        groups
            .extend(images.levels.windows(2).map(|pair| group(&self.reduce, &[view(4, &pair[0]), view(5, &pair[1])])));
        self.groups = Some((depth.clone(), groups));
    }
}
