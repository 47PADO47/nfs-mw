//! Textured world particles, depth-tested with reusable per-batch vertex buffers.

use std::collections::HashMap;

use blackbox_gfx::{BlendMode, EffectVertex, TextureHandle, TexturedEffect};

use crate::batch::Batch;
use crate::world::{DEPTH_FORMAT, EFFECT_VERTEX_ATTRIBUTES, WorldBindings, write_mask};

/// Textured world particles: one batch per [`TexturedEffect`], depth-tested against the scene depth
/// (reverse-Z, no depth writes), drawn with the texture bind group the caller keeps for each handle.
/// Entry points, indexed as `pipelines[usize::from(blend == BlendMode::Additive)]`.
const ENTRIES: [&str; 2] = ["fs_textured_alpha", "fs_textured"];

pub struct TexturedEffects {
    /// The plain and sRGB-target shader modules, built once and shared by every format's pipelines.
    shaders: HashMap<bool, wgpu::ShaderModule>,
    layout: wgpu::PipelineLayout,
    /// Per target format, built the first time the effects are drawn into it.
    pipelines: HashMap<wgpu::TextureFormat, [wgpu::RenderPipeline; 2]>,
    format: wgpu::TextureFormat,
    batches: Vec<(Batch, TextureHandle, BlendMode)>,
    count: usize,
}

impl TexturedEffects {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, bindings: &WorldBindings) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("textured world effects"),
            bind_group_layouts: &[Some(&bindings.globals_layout), Some(&bindings.texture_layout)],
            immediate_size: 0,
        });
        let mut effects =
            Self { shaders: HashMap::new(), layout, pipelines: HashMap::new(), format, batches: Vec::new(), count: 0 };
        effects.use_format(device, format);
        effects
    }

    /// Draw into `format` from now on, building its pipelines the first time.
    pub fn use_format(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        self.format = format;
        if !self.pipelines.contains_key(&format) {
            let pipelines = self.build(device, format);
            self.pipelines.insert(format, pipelines);
        }
    }

    fn build(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) -> [wgpu::RenderPipeline; 2] {
        let srgb = crate::world::is_srgb(format);
        let shader = self.shaders.entry(srgb).or_insert_with(|| {
            let source = match srgb {
                true => crate::EFFECTS_WGSL_SRGB,
                false => crate::EFFECTS_WGSL,
            };
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("textured world effects"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        });
        std::array::from_fn(|i| {
            let blend = match i {
                0 => wgpu::BlendState::ALPHA_BLENDING,
                _ => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                },
            };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("textured world effects"),
                layout: Some(&self.layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<EffectVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &EFFECT_VERTEX_ATTRIBUTES,
                    })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some(ENTRIES[i]),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: write_mask(format),
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    /// Replace the batches with `layers`, one per textured effect (reusing the buffers).
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layers: &[TexturedEffect]) {
        self.count = layers.len();
        for (i, layer) in layers.iter().enumerate() {
            if i == self.batches.len() {
                self.batches.push((Batch::new(device), layer.texture, layer.blend));
            }
            let (batch, texture, blend) = &mut self.batches[i];
            *texture = layer.texture;
            *blend = layer.blend;
            batch.upload(device, queue, &layer.vertices);
        }
    }

    /// Draw every batch whose texture `texture` can resolve. Bind group 0 (the globals) must already be
    /// set on `pass`; group 1 is set here to the bind group `texture` returns, which must have been made
    /// with [`WorldBindings::texture_layout`]. A handle it cannot resolve is skipped.
    pub fn draw<'t>(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        texture: impl Fn(TextureHandle) -> Option<&'t wgpu::BindGroup>,
    ) {
        let Some(pipelines) = self.pipelines.get(&self.format) else { return };
        for (batch, handle, blend) in self.batches.iter().take(self.count) {
            if batch.count() == 0 {
                continue;
            }
            let Some(texture) = texture(*handle) else { continue };
            pass.set_pipeline(&pipelines[usize::from(*blend == BlendMode::Additive)]);
            pass.set_bind_group(1, texture, &[]);
            pass.set_vertex_buffer(0, batch.buffer().slice(..));
            pass.draw(0..batch.count(), 0..1);
        }
    }
}
