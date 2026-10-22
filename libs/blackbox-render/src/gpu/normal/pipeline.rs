//! Tangent-space material layout and blend/depth pipelines.

use super::super::{
    draw_list::instance_view::instance_layout, pipelines::VERTEX_ATTRIBUTES, resources::DEPTH_FORMAT,
    targets::write_mask,
};
use blackbox_gfx::BlendMode;

pub(super) fn material_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let texture = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let uniform = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("normal scenery material"),
        entries: &[
            texture(0),
            texture(1),
            texture(2),
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            uniform(4),
            uniform(5),
            texture(6),
        ],
    })
}

/// `store`: the shader reads its instances from the instance store, through a slot per instance.
#[allow(clippy::too_many_arguments)]
pub(in crate::gpu) fn create(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    mode: BlendMode,
    tangent: bool,
    store: bool,
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    let (entry, blend, depth_write) = match mode {
        BlendMode::Opaque => ("fs_opaque", None, true),
        BlendMode::AlphaTest => ("fs_alpha_test", None, true),
        BlendMode::AlphaBlend => ("fs_blend", Some(wgpu::BlendState::ALPHA_BLENDING), false),
        BlendMode::Additive => (
            "fs_blend",
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::SrcAlpha,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            }),
            false,
        ),
    };
    let mut buffers = vec![
        Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<blackbox_gfx::Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }),
        Some(instance_layout(store)),
    ];
    if tangent {
        buffers.push(Some(wgpu::VertexBufferLayout {
            array_stride: 16,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![8 => Float32x4],
        }));
    }
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("normal scenery"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &buffers,
        },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(depth_write),
            depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: write_mask(format) })],
        }),
        multiview_mask: None,
        cache,
    })
}
