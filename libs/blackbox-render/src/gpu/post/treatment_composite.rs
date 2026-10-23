//! Full-size colour composition, isolated from the shared filter layout.

use super::{PassContext, PassIo};
use blackbox_gfx::{ColourTreatment, PostSettings};
use blackbox_gpu_passes::BindCache;
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
    /// `x` is the exposure of the fused tone mapping.
    tonemap: [f32; 4],
}

fn parameters(settings: &PostSettings, profile: Option<&ColourTreatment>) -> Parameters {
    let mut out = Parameters {
        tint: [0.0; 4],
        luminance: [0.0; 4],
        strengths: [0.0; 4],
        black_curve: [0.0; 4],
        colour_curve: [0.0; 4],
        tonemap: [0.0; 4],
    };
    let Some(p) = profile.copied() else { return out };
    let p = settings.colour_treatment_override.map_or(p, |replacement| replacement.apply(p));
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
    params: Parameters,
    /// Per output format and whether the tone mapping is fused into the draw.
    pipelines: HashMap<(wgpu::TextureFormat, bool), wgpu::RenderPipeline>,
    /// Bind groups per (input, blur) view pair. The gain map never changes for one composite (a new profile
    /// builds a new one), and the sampler and parameter buffer are fixed, so a steady frame makes none.
    bind_groups: BindCache<wgpu::TextureView, wgpu::BindGroup>,
    /// The exposure of the tone mapping run as part of the draw, when the next pass of the chain was fused in.
    tonemap: Option<f32>,
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
        let params = parameters(settings, profile);
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("colour treatment parameters"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            shader,
            layout,
            pipeline_layout,
            sampler,
            uniform,
            params,
            pipelines: HashMap::new(),
            bind_groups: BindCache::default(),
            tonemap: None,
            cache: cache.cloned(),
        }
    }

    /// Set the HDR flag (`tint.w`): with it the shader keeps positive scene values above 1 instead of clamping them.
    pub(super) fn set_hdr(&mut self, hdr: bool) {
        self.params.tint[3] = u8::from(hdr) as f32;
    }

    /// Run the tone mapping with `exposure` as part of the next draw (`None`: not at all).
    pub(super) fn set_tonemap(&mut self, exposure: Option<f32>) {
        self.tonemap = exposure;
        self.params.tonemap[0] = exposure.unwrap_or(0.0);
    }

    pub(super) fn draw(
        &mut self,
        ctx: &PassContext<'_>,
        io: &PassIo<'_>,
        images: [&wgpu::TextureView; 2],
        encoder: &mut wgpu::CommandEncoder,
    ) {
        ctx.queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&self.params));
        // The gain map (`images[1]`) is fixed for this composite, so the other two views are the whole key.
        let group = self.bind_groups.get_or_insert_with(io.input, images[0], || {
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("colour treatment inputs"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(io.input) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(images[0]) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(images[1]) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 4, resource: self.uniform.as_entire_binding() },
                ],
            })
        });
        let fused = self.tonemap.is_some();
        let entry = match fused {
            true => "fs_main_tonemap",
            false => "fs_main",
        };
        let pipeline = self.pipelines.entry((io.output_format, fused)).or_insert_with(|| {
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
                    entry_point: Some(entry),
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
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::ColourTreatmentOverride;
    #[test]
    fn parameter_replacements_keep_authored_curves_luminance_and_strength() {
        let profile = ColourTreatment {
            tint: [1.0; 3],
            luminance: [0.2, 0.3, 0.4],
            desaturation: 0.1,
            black_intensity: 0.2,
            colour_intensity: 0.3,
            black_curve: [0.1, 0.2, 0.3, 0.4],
            colour_curve: [0.4, 0.3, 0.2, 0.1],
        };
        let replacement = ColourTreatmentOverride {
            tint: [0.12, 0.34, 0.56],
            desaturation: 0.7,
            black_intensity: 0.8,
            colour_intensity: 0.9,
        };
        let settings =
            PostSettings { colour_treatment: 0.65, colour_treatment_override: Some(replacement), ..Default::default() };
        let actual = parameters(&settings, Some(&profile));
        assert_eq!(actual.tint[..3], replacement.tint);
        assert_eq!(actual.strengths, [0.7, 0.8, 0.9, 0.65]);
        assert_eq!(actual.luminance[..3], profile.luminance);
        assert_eq!(actual.black_curve, profile.black_curve);
        assert_eq!(actual.colour_curve, profile.colour_curve);
        assert_eq!(parameters(&PostSettings::default(), Some(&profile)).tint[..3], profile.tint);
        assert_eq!(parameters(&settings, None).strengths, [0.0; 4]);
        let invalid = PostSettings {
            colour_treatment_override: Some(ColourTreatmentOverride { desaturation: f32::NAN, ..replacement }),
            ..settings
        };
        assert!(invalid.sanitized().colour_treatment_override.is_none());
    }
}
