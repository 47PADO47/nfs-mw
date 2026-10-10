//! The scene pipelines, one per blend mode × shading model, and the vertex layouts.

use super::instances::INSTANCE_LAYOUT;
use super::resources::{DEPTH_FORMAT, Shared};
use crate::{BlendMode, GlossyMaterialHandle, Shading, Vertex};

/// Draw order: opaque and alpha-tested first, blended last.
pub(super) const BLEND_ORDER: [BlendMode; 4] =
    [BlendMode::Opaque, BlendMode::AlphaTest, BlendMode::AlphaBlend, BlendMode::Additive];
pub(super) const SHADINGS: [Shading; 4] =
    [Shading::Lit, Shading::Prelit, Shading::Sky, Shading::Glossy(GlossyMaterialHandle::ANY)];

pub(super) struct Pipelines {
    /// Indexed by `index(blend, shading)`.
    pipelines: Vec<wgpu::RenderPipeline>,
}

fn index(blend: BlendMode, shading: Shading) -> usize {
    let b = BLEND_ORDER.iter().position(|&m| m == blend).unwrap_or(0);
    let s = SHADINGS.iter().position(|&m| m.same_pipeline(shading)).unwrap_or(0);
    b * SHADINGS.len() + s
}

const ADDITIVE: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent::OVER,
};

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Unorm8x4, 3 => Float32x2];

impl Pipelines {
    pub(super) fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat, shared: &Shared) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/scene.wgsl").into()),
        });
        let glossy_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glossy shader"),
            source: wgpu::ShaderSource::Wgsl(super::glossy::SHADER.into()),
        });
        let glossy_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("glossy"),
            bind_group_layouts: &[
                Some(&shared.globals_layout),
                Some(&shared.texture_layout),
                Some(&shared.glossy.scene),
                Some(&shared.glossy.material),
            ],
            immediate_size: 0,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene"),
            bind_group_layouts: &[Some(&shared.globals_layout), Some(&shared.texture_layout)],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        };

        let make = |blend_mode: BlendMode, shading: Shading| {
            let (entry, blend, depth_write) = match blend_mode {
                BlendMode::Opaque => ("fs_opaque", None, true),
                BlendMode::AlphaTest => ("fs_alpha_test", None, true),
                BlendMode::AlphaBlend => ("fs_blend", Some(wgpu::BlendState::ALPHA_BLENDING), false),
                BlendMode::Additive => ("fs_blend", Some(ADDITIVE), false),
            };
            let label = format!("{blend_mode:?} {shading:?}");
            let prelit = matches!(shading, Shading::Prelit | Shading::Sky);
            let constants =
                [("PRELIT", f64::from(u8::from(prelit))), ("FOG", f64::from(u8::from(shading != Shading::Sky)))];
            // The glossy shader has its own module, layout and no overrides.
            let glossy = matches!(shading, Shading::Glossy(_));
            let (module, pipeline_layout, constants): (_, _, &[(&str, f64)]) =
                if glossy { (&glossy_shader, &glossy_layout, &[]) } else { (&shader, &layout, &constants) };
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&label),
                layout: Some(pipeline_layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(vertex_layout.clone()), Some(INSTANCE_LAYOUT)],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    // The games' culling per effect is not mapped yet; draw both sides.
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(depth_write),
                    // Reverse Z: near = 1, far = 0.
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions { constants, ..Default::default() },
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };

        let pipelines =
            BLEND_ORDER.iter().flat_map(|&b| SHADINGS.iter().map(move |&s| (b, s))).map(|(b, s)| make(b, s)).collect();
        Self { pipelines }
    }

    pub(super) fn get(&self, blend: BlendMode, shading: Shading) -> &wgpu::RenderPipeline {
        &self.pipelines[index(blend, shading)]
    }
}
