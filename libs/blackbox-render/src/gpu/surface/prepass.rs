//! Camera-space depth and world normals, excluding sky and blended surfaces.
use super::targets::{NORMAL_FORMAT, Targets};
use crate::gpu::draw_list::{instance_store, instance_view};
use crate::gpu::{Renderer, pipelines::VERTEX_ATTRIBUTES};
use blackbox_gfx::{BlendMode, FrameParams, Instance, Shading};

pub(super) struct Prepass {
    opaque: wgpu::RenderPipeline,
    cutout: wgpu::RenderPipeline,
    glossy_cutout: wgpu::RenderPipeline,
    tree_cutout: wgpu::RenderPipeline,
    prelit_cutout: wgpu::RenderPipeline,
    matrix: wgpu::Buffer,
    group: wgpu::BindGroup,
    /// Instances are read from the instance store, bound at group 2.
    store: bool,
}

impl Prepass {
    pub(super) fn new(
        device: &wgpu::Device,
        textures: &wgpu::BindGroupLayout,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("contact camera matrix"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let matrix = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("contact camera matrix"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("contact camera matrix"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: matrix.as_entire_binding() }],
        });
        let store = instance_store::supported(device);
        let instances = store.then(|| instance_store::layout(device));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("contact prepass"),
            bind_group_layouts: &[Some(&layout), Some(textures), instances.as_ref()][..2 + usize::from(store)],
            immediate_size: 0,
        });
        let source =
            instance_view::instance_source(include_str!("../../shaders/contact_prepass.wgsl"), store.then_some(2));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("contact prepass"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
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
                        Some(instance_view::instance_layout(store)),
                    ],
                },
                primitive: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
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
                        format: NORMAL_FORMAT,
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
            tree_cutout: make("fs_tree_cutout"),
            prelit_cutout: make("fs_prelit_cutout"),
            matrix,
            group,
            store,
        }
    }

    pub(super) fn encode(
        &mut self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        targets: &Targets,
        frame: &FrameParams,
        instances: &[Instance],
    ) {
        renderer.queue.write_buffer(
            &self.matrix,
            0,
            bytemuck::cast_slice(&renderer.world_to_clip(frame).to_cols_array()),
        );
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("contact depth normal prepass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.normals,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.group, &[]);
        let store = &renderer.draw_list.store;
        if let Some(instances) = store.bind_group().filter(|_| self.store) {
            pass.set_bind_group(2, instances, &[]);
        }
        pass.set_vertex_buffer(1, renderer.instances.buffer.slice(store.main_offset()..));
        let mut bound_pipeline: Option<&wgpu::RenderPipeline> = None;
        let mut bound_texture: Option<&wgpu::BindGroup> = None;
        let mut first = 0;
        while first < instances.len() {
            let mesh_handle = instances[first].mesh;
            let end = first + instances[first..].iter().take_while(|i| i.mesh == mesh_handle).count();
            if let Some(mesh) = renderer.meshes.get(mesh_handle.raw()) {
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                for draw in &mesh.draws {
                    if matches!(draw.shading, Shading::Sky | Shading::SkyLayer(_))
                        || !matches!(draw.blend, BlendMode::Opaque | BlendMode::AlphaTest)
                    {
                        continue;
                    }
                    if let Shading::Glossy(material) = draw.shading
                        && renderer.glossy.as_ref().is_none_or(|g| !g.has_material(material))
                    {
                        continue;
                    }
                    // Original cutouts test the composed vertex-ramp alpha. The
                    // texture-only prepass cannot reproduce that material-dependent value.
                    if draw.blend == BlendMode::AlphaTest
                        && let Shading::Glossy(material) = draw.shading
                        && renderer.glossy.as_ref().is_some_and(|g| g.original_material(material))
                    {
                        continue;
                    }
                    let pipeline = match draw.blend {
                        BlendMode::AlphaTest if renderer.reflective_surface(draw.shading) => &self.opaque,
                        BlendMode::AlphaTest if matches!(draw.shading, Shading::Glossy(_)) => &self.glossy_cutout,
                        BlendMode::AlphaTest
                            if renderer.experiments.original_world_materials && draw.shading == Shading::Tree =>
                        {
                            &self.tree_cutout
                        }
                        BlendMode::AlphaTest
                            if renderer.experiments.original_world_materials
                                && matches!(draw.shading, Shading::WorldWindow(_)) =>
                        {
                            &self.glossy_cutout
                        }
                        BlendMode::AlphaTest
                            if renderer.experiments.original_world_materials
                                && draw.shading == Shading::WorldPrelit =>
                        {
                            &self.prelit_cutout
                        }
                        BlendMode::AlphaTest => &self.cutout,
                        _ => &self.opaque,
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
                    pass.draw_indexed(
                        draw.first_index..draw.first_index + draw.index_count,
                        draw.base_vertex,
                        first as u32..end as u32,
                    );
                }
            }
            first = end;
        }
    }
}
