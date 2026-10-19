//! The motion-vector pass: the opaque and alpha-tested surfaces drawn again over the finished depth buffer,
//! writing where each point was last frame minus where it is now.
//!
//! The camera's own motion comes from the previous frame's matrix, an object's from its previous transform
//! (`history`); an object without a key has only the camera's. Depth is read, not written, so a pixel gets the
//! vector of the surface the colour pass left visible.

use super::TemporalFrame;
use crate::gpu::Renderer;
use crate::gpu::instances::INSTANCE_LAYOUT;
use crate::gpu::pipelines::VERTEX_ATTRIBUTES;
use crate::gpu::resources::DEPTH_FORMAT;
use crate::gpu::targets::MOTION_FORMAT;
use blackbox_gfx::{BlendMode, Instance, Shading};

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
    bias: [f32; 4],
}

pub(in crate::gpu) struct MotionPass {
    opaque: wgpu::RenderPipeline,
    cutout: wgpu::RenderPipeline,
    glossy_cutout: wgpu::RenderPipeline,
    camera: wgpu::Buffer,
    group: wgpu::BindGroup,
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("motion"),
            bind_group_layouts: &[Some(&layout), Some(textures)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("motion"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/motion.wgsl").into()),
        });
        let make = |entry| {
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
                        Some(INSTANCE_LAYOUT),
                        Some(PREVIOUS_LAYOUT),
                    ],
                },
                // Both sides, like the colour pass.
                primitive: Default::default(),
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
            opaque: make("fs_opaque"),
            cutout: make("fs_cutout"),
            glossy_cutout: make("fs_glossy_cutout"),
            camera,
            group,
        }
    }

    /// Draw the vectors of `instances` into `motion`, against `depth`. `previous` has the previous matrix of
    /// each instance, in the same order.
    pub(in crate::gpu) fn encode(
        &mut self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        targets: (&wgpu::TextureView, &wgpu::TextureView),
        frame: &TemporalFrame,
        instances: &[Instance],
        previous: &wgpu::Buffer,
    ) {
        let (motion, depth) = targets;
        let camera = Camera {
            jittered: (frame.jitter_matrix * frame.unjittered).to_cols_array(),
            current: frame.unjittered.to_cols_array(),
            previous: frame.previous.to_cols_array(),
            bias: [renderer.upscale.texture_lod_bias, 0.0, 0.0, 0.0],
        };
        renderer.queue.write_buffer(&self.camera, 0, bytemuck::bytes_of(&camera));
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
        pass.set_bind_group(0, &self.group, &[]);
        pass.set_vertex_buffer(1, renderer.instances.buffer.slice(..));
        pass.set_vertex_buffer(2, previous.slice(..));
        let mut bound_pipeline: Option<&wgpu::RenderPipeline> = None;
        let mut bound_texture: Option<&wgpu::BindGroup> = None;
        let mut first = 0;
        while first < instances.len() {
            let mesh_handle = instances[first].mesh;
            let end = first + instances[first..].iter().take_while(|i| i.mesh == mesh_handle).count();
            let Some(mesh) = renderer.meshes.get(mesh_handle.raw()) else {
                first = end;
                continue;
            };
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
                    _ => &self.opaque,
                };
                if bound_pipeline.is_none_or(|bound| !std::ptr::eq(bound, pipeline)) {
                    pass.set_pipeline(pipeline);
                    bound_pipeline = Some(pipeline);
                }
                let slot = draw.texture.map(|t| renderer.redirects.get(&t.raw()).copied().unwrap_or(t.raw()));
                let Some(texture) = slot.and_then(|s| renderer.textures.get(s)).or(renderer.textures.get(0)) else {
                    continue;
                };
                if bound_texture.is_none_or(|bound| !std::ptr::eq(bound, texture)) {
                    pass.set_bind_group(1, texture, &[]);
                    bound_texture = Some(texture);
                }
                pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    draw.base_vertex,
                    first as u32..end as u32,
                );
            }
            first = end;
        }
    }
}
