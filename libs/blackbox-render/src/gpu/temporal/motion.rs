//! The motion-vector pass: where each point was last frame minus where it is now.
//!
//! One full-screen draw gives every pixel the camera's motion: it rebuilds the pixel's world position from the
//! finished depth buffer and projects it with this frame's and the previous frame's matrices. That is the whole
//! answer for everything that stood still. Over it the pass draws again only the instances whose transform
//! changed since the last frame (`history`), the opaque and alpha-tested surfaces, with their previous matrix.
//! An instance without a key, or a new one, has only the camera's motion. Depth is read, not written, so a
//! pixel gets the vector of the surface the colour pass left visible.

use std::ops::Range;

use super::TemporalFrame;
use super::history::History;
use crate::gpu::Renderer;
use crate::gpu::cull::{FACES, Face};
use crate::gpu::draw_list::{instance_store, instance_view, run};
use crate::gpu::pipelines::VERTEX_ATTRIBUTES;
use crate::gpu::resources::DEPTH_FORMAT;
use crate::gpu::targets::MOTION_FORMAT;
use blackbox_gfx::{BlendMode, Instance, Shading};
use blackbox_gpu_passes::BindCache;
use glam::DMat4;

/// Last frame's model matrices: one column per attribute, after the current ones.
const PREVIOUS_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Float32x4];

const PREVIOUS_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &PREVIOUS_ATTRIBUTES,
};

/// The uniform block of `motion.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Camera {
    jittered: [f32; 16],
    current: [f32; 16],
    previous: [f32; 16],
    /// The inverse of `jittered`, to rebuild world positions from depth.
    inverse: [f32; 16],
    bias: [f32; 4],
}

/// The inverse of `matrix`, computed in double precision: the world positions the fill draw rebuilds from depth
/// are only as exact as this.
pub(super) fn inverse(matrix: glam::Mat4) -> [f32; 16] {
    let wide = DMat4::from_cols_array(&matrix.to_cols_array().map(f64::from));
    wide.inverse().to_cols_array().map(|value| value as f32)
}

/// The runs of consecutive `true` entries of `moved` inside `start..end`, as instance ranges. Instances past
/// the end of `moved` count as still.
pub(super) fn moved_runs(moved: &[bool], start: usize, end: usize) -> impl Iterator<Item = Range<u32>> + '_ {
    let mut cursor = start;
    let is_moved = move |index: usize| moved.get(index).copied().unwrap_or(false);
    std::iter::from_fn(move || {
        while cursor < end && !is_moved(cursor) {
            cursor += 1;
        }
        if cursor >= end {
            return None;
        }
        let begin = cursor;
        while cursor < end && is_moved(cursor) {
            cursor += 1;
        }
        Some(begin as u32..cursor as u32)
    })
}

pub(in crate::gpu) struct MotionPass {
    /// The full-screen draw of the camera's motion.
    fill: wgpu::RenderPipeline,
    fill_layout: wgpu::BindGroupLayout,
    /// The camera block with one depth view each: a resize makes a new depth buffer, and so a new group.
    fill_groups: BindCache<wgpu::TextureView, wgpu::BindGroup>,
    /// One per face setting, in `FACES` order: the vectors belong to the faces the colour pass drew.
    opaque: [wgpu::RenderPipeline; 3],
    cutout: wgpu::RenderPipeline,
    glossy_cutout: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    group: wgpu::BindGroup,
    /// This frame's instances are read from the instance store, bound at group 2.
    store: bool,
}

impl MotionPass {
    pub(in crate::gpu) fn new(
        device: &wgpu::Device,
        textures: &wgpu::BindGroupLayout,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        Self::with_format(device, textures, cache, MOTION_FORMAT)
    }

    /// Effects may retain previous view depth and coverage in the extra two channels.
    pub(in crate::gpu) fn with_format(
        device: &wgpu::Device,
        textures: &wgpu::BindGroupLayout,
        cache: Option<&wgpu::PipelineCache>,
        format: wgpu::TextureFormat,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion camera"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("motion camera"),
            size: size_of::<Camera>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("motion camera"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera.as_entire_binding() }],
        });
        let store = instance_store::supported(device);
        let instances = store.then(|| instance_store::layout(device));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion"),
            bind_group_layouts: &[Some(&layout), Some(textures), instances.as_ref()][..2 + usize::from(store)],
            immediate_size: 0,
        });
        let source = instance_view::instance_source(include_str!("../../shaders/motion.wgsl"), store.then_some(2));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("motion"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let fill_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("motion fill"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let fill_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion fill"),
            bind_group_layouts: &[Some(&fill_layout)],
            immediate_size: 0,
        });
        // The pass has the scene's depth attached read-only, so the fill pipeline names the format and
        // neither tests nor writes it.
        let fill = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("motion fill"),
            layout: Some(&fill_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fill"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_fill"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache,
        });
        let make = |entry, face: Face| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        Some(wgpu::VertexBufferLayout {
                            array_stride: size_of::<crate::Vertex>() as u64,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &VERTEX_ATTRIBUTES,
                        }),
                        Some(instance_view::instance_layout(store)),
                        Some(PREVIOUS_LAYOUT),
                    ],
                },
                // The faces the colour pass draws.
                primitive: wgpu::PrimitiveState { cull_mode: face.cull_mode(), ..Default::default() },
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
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache,
            })
        };
        Self {
            fill,
            fill_layout,
            fill_groups: BindCache::default(),
            opaque: FACES.map(|face| make("fs_opaque", face)),
            cutout: make("fs_cutout", Face::Both),
            glossy_cutout: make("fs_glossy_cutout", Face::Both),
            camera,
            group,
            store,
        }
    }

    /// Draw the vectors into `motion`, against `depth`: the camera's motion for every pixel, then the
    /// `instances` that `history` says moved, with their previous matrices.
    pub(super) fn encode(
        &mut self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&wgpu::TextureView, &wgpu::TextureView),
        frame: &TemporalFrame,
        instances: &[Instance],
        history: &History,
    ) {
        let (motion, depth) = targets;
        let moved = history.moved();
        let camera = Camera {
            jittered: (frame.jitter_matrix * frame.unjittered).to_cols_array(),
            current: frame.unjittered.to_cols_array(),
            previous: frame.previous.to_cols_array(),
            inverse: inverse(frame.jitter_matrix * frame.unjittered),
            bias: [renderer.upscale.texture_lod_bias, 0.0, 0.0, 0.0],
        };
        renderer.queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(&camera));
        let fill_group = self.fill_groups.get_or_insert_with(depth, depth, || {
            renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("motion fill"),
                layout: &self.fill_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.camera.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(depth) },
                ],
            })
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("motion vectors"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: motion,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: None,
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.fill);
        pass.set_bind_group(0, fill_group, &[]);
        pass.draw(0..3, 0..1);
        if !moved.contains(&true) {
            return;
        }
        pass.set_bind_group(0, &self.group, &[]);
        let store = &renderer.draw_list.store;
        if let Some(instances) = store.bind_group().filter(|_| self.store) {
            pass.set_bind_group(2, instances, &[]);
        }
        pass.set_vertex_buffer(1, renderer.instances.buffer.slice(store.main_offset()..));
        pass.set_vertex_buffer(2, history.buffer.buffer.slice(..));
        let mut bound_pipeline: Option<&wgpu::RenderPipeline> = None;
        let mut bound_texture: Option<&wgpu::BindGroup> = None;
        // The perspective's determinant is positive, so the camera mirrors exactly when this does.
        let mirrored_view = frame.unjittered.determinant() < 0.0;
        let mut first = 0;
        while first < instances.len() {
            let mesh = renderer.meshes.get(instances[first].mesh.raw());
            let (end, mirrored) = run(instances, first, mesh.is_some_and(|m| m.draws.iter().any(|d| !d.two_sided)));
            let start = first;
            first = end;
            if moved_runs(moved, start, end).next().is_none() {
                continue;
            }
            let Some(mesh) = mesh else { continue };
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
            for draw in &mesh.draws {
                if !matches!(draw.blend, BlendMode::Opaque | BlendMode::AlphaTest) {
                    continue;
                }
                if let Shading::Glossy(material) = draw.shading
                    && renderer.glossy.as_ref().is_none_or(|g| !g.has_material(material))
                {
                    continue;
                }
                let pipeline = match draw.blend {
                    BlendMode::AlphaTest if matches!(draw.shading, Shading::Glossy(_)) => &self.glossy_cutout,
                    BlendMode::AlphaTest => &self.cutout,
                    _ => &self.opaque[Face::of(draw, mirrored).seen(mirrored_view).index()],
                };
                if bound_pipeline.is_none_or(|bound| !std::ptr::eq(bound, pipeline)) {
                    pass.set_pipeline(pipeline);
                    bound_pipeline = Some(pipeline);
                }
                let slot = draw.texture.map(|t| renderer.redirects.resolve(t.raw()));
                let Some(texture) = slot.and_then(|s| renderer.textures.get(s)).or(renderer.textures.get(0)) else {
                    continue;
                };
                if bound_texture.is_none_or(|bound| !std::ptr::eq(bound, texture)) {
                    pass.set_bind_group(1, texture, &[]);
                    bound_texture = Some(texture);
                }
                for changed in moved_runs(moved, start, end) {
                    pass.draw_indexed(draw.first_index..draw.first_index + draw.index_count, draw.base_vertex, changed);
                }
            }
        }
    }
}
