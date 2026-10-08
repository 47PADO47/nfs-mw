//! Textured billboard batches of the effects layer: one draw per batch, alpha-blended or additive.

use std::collections::HashMap;
use std::ops::Range;

use super::Renderer;
use super::effects::{ATTRIBUTES, Batch};
use super::resources::{DEPTH_FORMAT, Shared};
use super::slots::Slots;
use crate::{EffectLayer, EffectVertex, TextureHandle};

/// One draw: a run of the shared vertex buffer with one texture and one blend.
struct Run {
    vertices: Range<u32>,
    texture: Option<TextureHandle>,
    additive: bool,
}

pub(super) struct Sprites {
    batch: Batch,
    runs: Vec<Run>,
    /// `[alpha, additive]`.
    pipelines: [wgpu::RenderPipeline; 2],
    scratch: Vec<EffectVertex>,
}

impl Sprites {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, shared: &Shared) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprites"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sprites.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprites"),
            bind_group_layouts: &[Some(&shared.globals_layout), Some(&shared.texture_layout)],
            immediate_size: 0,
        });
        let make = |additive: bool| {
            let blend = match additive {
                true => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                },
                false => wgpu::BlendState::ALPHA_BLENDING,
            };
            let constants = [("ADDITIVE", f64::from(u8::from(additive)))];
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(if additive { "sprites additive" } else { "sprites alpha" }),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<EffectVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &ATTRIBUTES,
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
                    module: &shader,
                    entry_point: Some("fs_sprite"),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &constants,
                        ..Default::default()
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        Self { batch: Batch::new(device), runs: Vec::new(), pipelines: [make(false), make(true)], scratch: Vec::new() }
    }

    /// Replace the batches drawn every frame until replaced again.
    pub(super) fn set(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, layer: &EffectLayer) {
        self.runs.clear();
        self.scratch.clear();
        for batch in layer.sprites.iter().filter(|b| !b.vertices.is_empty()) {
            let first = self.scratch.len() as u32;
            self.scratch.extend_from_slice(&batch.vertices);
            self.runs.push(Run {
                vertices: first..self.scratch.len() as u32,
                texture: batch.texture,
                additive: batch.additive,
            });
        }
        self.batch.upload(device, queue, &self.scratch);
    }

    pub(super) fn capacity(&self) -> usize {
        self.batch.capacity
    }

    pub(super) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        textures: &Slots<wgpu::BindGroup>,
        redirects: &HashMap<usize, usize>,
    ) {
        for run in &self.runs {
            let slot = run.texture.map(|t| redirects.get(&t.0).copied().unwrap_or(t.0));
            let Some(texture) = slot.and_then(|s| textures.get(s)).or(textures.get(0)) else { continue };
            pass.set_pipeline(&self.pipelines[usize::from(run.additive)]);
            pass.set_bind_group(1, texture, &[]);
            pass.set_vertex_buffer(0, self.batch.buffer.slice(..));
            pass.draw(run.vertices.clone(), 0..1);
        }
    }
}

impl Renderer {
    /// Allocated capacity in vertices of the textured sprite buffer.
    pub fn sprite_capacity(&self) -> usize {
        self.effects.sprites.capacity()
    }
}
