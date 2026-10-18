//! Small linear-depth hierarchy; near taps still use the full-resolution depth.
pub(super) struct DepthPyramid {
    pub size: (u32, u32),
    pub xe: bool,
    pub view: wgpu::TextureView,
    levels: Vec<wgpu::TextureView>,
    inputs: Option<(wgpu::TextureView, Vec<wgpu::BindGroup>)>,
}

impl DepthPyramid {
    pub fn new(device: &wgpu::Device, scene_size: (u32, u32)) -> Self {
        Self::with_method(device, scene_size, false)
    }

    pub fn with_method(device: &wgpu::Device, scene_size: (u32, u32), xe: bool) -> Self {
        let size = ((scene_size.0 / 2).max(1), (scene_size.1 / 2).max(1));
        let count = (u32::BITS - size.0.max(size.1).leading_zeros()).min(match xe {
            true => 4,
            false => 3,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("AO linear depth hierarchy"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let levels = (0..count)
            .map(|i| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level: i,
                    mip_level_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        Self { size: scene_size, xe, view: texture.create_view(&Default::default()), levels, inputs: None }
    }

    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &super::pipelines::Pipelines,
        params: &wgpu::Buffer,
        depth: &wgpu::TextureView,
        fallback: &wgpu::TextureView,
    ) {
        if self.inputs.as_ref().is_none_or(|(stored, _)| stored != depth) {
            let groups = self
                .levels
                .iter()
                .enumerate()
                .map(|(i, view)| {
                    let previous = match i {
                        0 => fallback,
                        _ => &self.levels[i - 1],
                    };
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("AO depth reduction inputs"),
                        layout: &pipelines.depth_layout,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(depth) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(previous) },
                            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(view) },
                        ],
                    })
                })
                .collect();
            self.inputs = Some((depth.clone(), groups));
        }
        let groups = &self.inputs.as_ref().expect("depth inputs").1;
        for (i, group) in groups.iter().enumerate() {
            let pipeline = match i {
                0 => &pipelines.depth_first,
                _ => &pipelines.depth_reduce,
            };
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AO depth prefilter"),
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            let width = (self.view.texture().width() >> i).max(1);
            let height = (self.view.texture().height() >> i).max(1);
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
        }
    }
}
