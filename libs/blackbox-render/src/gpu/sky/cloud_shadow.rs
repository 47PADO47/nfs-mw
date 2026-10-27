//! A deterministic world-space receiver-plane cloud transmission field.
use super::super::resources::Globals;
use bytemuck::Zeroable;
use wgpu::util::DeviceExt;

/// Snap to a whole map texel so camera movement never shifts the sampled grid.
pub(in crate::gpu) fn anchor(value: f32) -> f32 {
    const TEXEL: f32 = 8000.0 / 256.0;
    (value / TEXEL).round() * TEXEL
}

pub(super) struct CloudShadow {
    globals: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl CloudShadow {
    pub fn new(
        device: &wgpu::Device,
        globals_layout: &wgpu::BindGroupLayout,
        material_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let globals = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cloud shadow globals"),
            contents: bytemuck::bytes_of(&Globals::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud shadow globals"),
            layout: globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }],
        });
        let source = format!(
            "{}\n{}",
            include_str!("../../shaders/cloud_shadow.wgsl"),
            include_str!("../../shaders/sky_clouds.wgsl")
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud shadow generation"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cloud shadow generation"),
            bind_group_layouts: &[Some(globals_layout), Some(material_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cloud shadow generation"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("shadow_vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("shadow_fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { globals, group, pipeline }
    }
    pub fn encode(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        material: &wgpu::BindGroup,
        globals: &Globals,
    ) {
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(globals));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cloud shadow transmission"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_bind_group(1, material, &[]);
        pass.draw(0..3, 0..1);
    }
}
