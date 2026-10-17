//! Full-size colour composition, isolated from the shared filter layout.

use super::{PassContext, PassIo};
use blackbox_gfx::{ColourTreatment, PostSettings};
use std::collections::HashMap;
use wgpu::util::DeviceExt;

pub(super) const SOURCE: &str = include_str!("../../shaders/treatment_composite.wgsl");

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Parameters {
    tint: [f32; 4],
    luminance: [f32; 4],
    strengths: [f32; 4],
    black_curve: [f32; 4],
    colour_curve: [f32; 4],
}

fn parameters(settings: &PostSettings, profile: Option<&ColourTreatment>) -> Parameters {
    let mut out = Parameters {
        tint: [0.0; 4],
        luminance: [0.0; 4],
        strengths: [0.0; 4],
        black_curve: [0.0; 4],
        colour_curve: [0.0; 4],
    };
    let Some(p) = profile else { return out };
    out.tint[..3].copy_from_slice(&p.tint);
    out.luminance[..3].copy_from_slice(&p.luminance);
    out.strengths = [p.desaturation, p.black_intensity, p.colour_intensity, settings.colour_treatment];
    out.black_curve = p.black_curve;
    out.colour_curve = p.colour_curve;
    out
}

pub(super) struct Composite {
    shader: wgpu::ShaderModule,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// Used by the pipeline built per output format.
    cache: Option<wgpu::PipelineCache>,
}

impl Composite {
    pub(super) fn new(
        device: &wgpu::Device,
        settings: &PostSettings,
        profile: Option<&ColourTreatment>,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("colour treatment composite"),
            source: wgpu::ShaderSource::Wgsl(SOURCE.into()),
        });
        let mut entries: Vec<_> = (0..3)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            })
            .collect();
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("colour treatment"),
            entries: &entries,
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("colour treatment"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("colour treatment parameters"),
            contents: bytemuck::bytes_of(&parameters(settings, profile)),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Self { shader, layout, pipeline_layout, sampler, uniform, pipelines: HashMap::new(), cache: cache.cloned() }
    }

    pub(super) fn draw(
        &mut self,
        ctx: &PassContext<'_>,
        io: &PassIo<'_>,
        images: [&wgpu::TextureView; 2],
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut entries: Vec<_> = std::iter::once(io.input)
            .chain(images)
            .enumerate()
            .map(|(i, view)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) });
        entries.push(wgpu::BindGroupEntry { binding: 4, resource: self.uniform.as_entire_binding() });
        let group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("colour treatment inputs"),
            layout: &self.layout,
            entries: &entries,
        });
        let pipeline = self.pipelines.entry(io.output_format).or_insert_with(|| {
            ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("colour treatment"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: io.output_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: self.cache.as_ref(),
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("colour treatment composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: io.output,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}
