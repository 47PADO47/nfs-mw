//! Write the real frame and a matching premultiplied UI resource in the output colour space.
pub(super) struct Composite {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl Composite {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("DLSS-G separate scene and UI"),
            entries: &[0, 1].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("DLSS-G composition"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let hdr10 = format == wgpu::TextureFormat::Rgb10a2Unorm;
        let formats = match hdr10 {
            true => vec![format, wgpu::TextureFormat::Rgba16Float, format],
            false => vec![format, format],
        };
        let targets: Vec<_> = formats
            .into_iter()
            .map(|format| Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL }))
            .collect();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("DLSS-G UI recomposition"),
            source: wgpu::ShaderSource::Wgsl(
                [include_str!("../../shaders/fg_composition.wgsl"), include_str!("../../shaders/fg_hdr10.wgsl")]
                    .concat()
                    .into(),
            ),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("DLSS-G composition"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(if hdr10 { "fs_hdr10" } else { "fs_main" }),
                targets: &targets,
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout }
    }

    pub(super) fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
        raw_ui: &wgpu::TextureView,
        outputs: &[&wgpu::TextureView],
    ) {
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("DLSS-G scene and UI inputs"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(scene) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(raw_ui) },
            ],
        });
        let attachments: Vec<_> = outputs
            .iter()
            .map(|view| {
                Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })
            })
            .collect();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("DLSS-G scene/UI composition"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}
