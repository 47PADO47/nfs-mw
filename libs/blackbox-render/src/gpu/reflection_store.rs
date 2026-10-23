//! Store the RH colour image in cube orientation, optionally fading finite capture boundaries.
use wgpu::util::DeviceExt;

pub(super) struct Store {
    pipeline: wgpu::RenderPipeline,
    groups: [wgpu::BindGroup; 6],
    controls: [wgpu::Buffer; 6],
    cached: [u32; 4],
}

impl Store {
    pub fn new(
        device: &wgpu::Device,
        color: &wgpu::TextureView,
        depths: &wgpu::TextureView,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cube store shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/reflection_store.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube store"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache,
        });
        let controls = std::array::from_fn(|_| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("cube store control"),
                contents: bytemuck::cast_slice(&[0.0_f32; 8]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        });
        let groups = std::array::from_fn(|face| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cube store inputs"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(color) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(depths) },
                    wgpu::BindGroupEntry { binding: 2, resource: controls[face].as_entire_binding() },
                ],
            })
        });
        Self { pipeline, groups, controls, cached: [0; 4] }
    }

    pub fn configure(&mut self, queue: &wgpu::Queue, far: Option<f32>, color: [f32; 3]) {
        let far = far.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(0.0);
        let parameters = match far > 0.0 {
            true => [color[0], color[1], color[2], far],
            false => [0.0; 4],
        };
        let key = parameters.map(f32::to_bits);
        if self.cached == key {
            return;
        }
        self.cached = key;
        for (face, buffer) in self.controls.iter().enumerate() {
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[parameters, [face as f32, 0.0, 0.0, 0.0]]));
        }
    }

    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, face: usize) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cube store"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[face], &[]);
        pass.draw(0..3, 0..1);
    }
}
