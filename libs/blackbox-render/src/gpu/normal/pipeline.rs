//! Tangent-space material layout and blend/depth pipelines.

use super::super::{
    cull::{FACES, Face},
    draw_list::instance_view::instance_layout,
    parallel,
    pipelines::{BLEND_ORDER, VERTEX_ATTRIBUTES},
    resources::DEPTH_FORMAT,
    targets::write_mask,
};
use blackbox_gfx::BlendMode;

/// One pipeline per blend mode in `BLEND_ORDER`, for each face setting built so far, in [`FACES`] order: the
/// two-sided set first, the culling sets once a single-sided draw exists.
pub(in crate::gpu) type FaceSets = Vec<[wgpu::RenderPipeline; 4]>;

/// Add the face settings `sets` lacks: the two-sided set, and with `culled` the culling sets. `make` builds the
/// pipeline of one blend mode and face setting; the missing ones are built on worker threads (see
/// [`parallel`]), as a cold driver cache spends up to half a second on each.
pub(in crate::gpu) fn fill_faces(
    sets: &mut FaceSets,
    culled: bool,
    make: impl Fn(BlendMode, Face) -> wgpu::RenderPipeline + Sync,
) {
    let wanted = match culled {
        true => FACES.len(),
        false => 1,
    };
    let missing = &FACES[sets.len().min(wanted)..wanted];
    if missing.is_empty() {
        return;
    }
    let keys: Vec<(Face, BlendMode)> = missing.iter().flat_map(|&face| BLEND_ORDER.map(|mode| (face, mode))).collect();
    let mut built = parallel::build(&keys, |&(face, mode)| make(mode, face)).into_iter();
    for _ in missing {
        let set: Vec<wgpu::RenderPipeline> = built.by_ref().take(BLEND_ORDER.len()).collect();
        let Ok(set) = set.try_into() else { return };
        sets.push(set);
    }
}

/// The pipeline of blend mode number `mode` (in `BLEND_ORDER`) for `face`, or the two-sided one when that face's
/// set is not built.
pub(in crate::gpu) fn pick(sets: &FaceSets, mode: usize, face: Face) -> Option<&wgpu::RenderPipeline> {
    sets.get(face.index()).or(sets.first()).and_then(|set| set.get(mode))
}

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
            uniform(7),
            texture(8),
            wgpu::BindGroupLayoutEntry {
                binding: 9,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 10,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 11,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
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
    (mode, face): (BlendMode, Face),
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
        primitive: wgpu::PrimitiveState { cull_mode: face.cull_mode(), ..Default::default() },
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
