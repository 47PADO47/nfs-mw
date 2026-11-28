//! Camera-space depth and world normals, excluding sky and blended surfaces.
use super::static_depth::{self, StaticDepth};
use super::targets::{NORMAL_FORMAT, Targets};
use crate::gpu::cull::{FACES, Face};
use crate::gpu::draw_list::{instance_store, instance_view, run};
use crate::gpu::meshes::GpuMesh;
use crate::gpu::{Renderer, pipelines::VERTEX_ATTRIBUTES};
use blackbox_gfx::{BlendMode, DrawRange, FrameParams, Instance, Shading};

pub(super) struct Prepass {
    /// One per face setting, in `FACES` order: the colour pass's `Equal` test needs the same faces.
    opaque: [wgpu::RenderPipeline; 3],
    cutout: wgpu::RenderPipeline,
    glossy_cutout: wgpu::RenderPipeline,
    window_cutout: wgpu::RenderPipeline,
    world_cutout: wgpu::RenderPipeline,
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
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
            size: 80,
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
                    ],
                },
                primitive: wgpu::PrimitiveState { cull_mode: face.cull_mode(), ..Default::default() },
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
            opaque: FACES.map(|face| make("fs_opaque", face)),
            cutout: make("fs_cutout", Face::Both),
            glossy_cutout: make("fs_glossy_cutout", Face::Both),
            window_cutout: make("fs_window_cutout", Face::Both),
            world_cutout: make("fs_world_cutout", Face::Both),
            tree_cutout: make("fs_tree_cutout", Face::Both),
            prelit_cutout: make("fs_prelit_cutout", Face::Both),
            matrix,
            group,
            store,
        }
    }

    /// Draw the camera depth and normals. With `snapshot`, the static scene is drawn first, its depth is copied,
    /// and the dynamic models ([`static_depth::dynamic`]) are drawn over it in a second pass.
    pub(super) fn encode(
        &mut self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        targets: &Targets,
        (frame, mip_bias): (&FrameParams, f32),
        instances: &[Instance],
        snapshot: Option<&mut StaticDepth>,
    ) {
        let mut camera = [0.0f32; 20];
        camera[..16].copy_from_slice(&renderer.world_to_clip(frame).to_cols_array());
        camera[16] = mip_bias;
        renderer.queue.write_buffer(&self.matrix, 0, bytemuck::cast_slice(&camera));
        let only = snapshot.is_some().then_some(false);
        let mut dynamic_runs = Vec::new();
        {
            let mut pass = self.begin(renderer, encoder, targets, true);
            let mut binder = Binder::default();
            let mut first = 0;
            while first < instances.len() {
                let mesh = renderer.meshes.get(instances[first].mesh.raw());
                let (end, mirrored) = run(instances, first, mesh.is_some_and(|m| m.draws.iter().any(|d| !d.two_sided)));
                if let Some(mesh) = mesh
                    && self.draw_run(&mut pass, &mut binder, (renderer, frame), (first, end, mirrored), mesh, only)
                {
                    dynamic_runs.push((first, end, mirrored));
                }
                first = end;
            }
        }
        let Some(snapshot) = snapshot else { return };
        if dynamic_runs.is_empty() {
            snapshot.invalidate();
            return;
        }
        snapshot.capture(&renderer.device, encoder, &targets.depth, targets.size);
        {
            let mut pass = self.begin(renderer, encoder, targets, false);
            let mut binder = Binder::default();
            for (first, end, mirrored) in dynamic_runs {
                let Some(mesh) = renderer.meshes.get(instances[first].mesh.raw()) else { continue };
                self.draw_run(&mut pass, &mut binder, (renderer, frame), (first, end, mirrored), mesh, Some(true));
            }
        }
        snapshot.mark(&renderer.device, encoder, &targets.depth);
    }

    /// A prepass over the targets: cleared for the first, loaded for the dynamic models drawn after the snapshot.
    fn begin<'e>(
        &self,
        renderer: &Renderer,
        encoder: &'e mut wgpu::CommandEncoder,
        targets: &Targets,
        clear: bool,
    ) -> wgpu::RenderPass<'e> {
        let (normals, depth) = match clear {
            true => (wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), wgpu::LoadOp::Clear(0.0)),
            false => (wgpu::LoadOp::Load, wgpu::LoadOp::Load),
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(match clear {
                true => "contact depth normal prepass",
                false => "contact depth normal prepass (dynamic models)",
            }),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &targets.normals,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: normals, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations { load: depth, store: wgpu::StoreOp::Store }),
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
        pass
    }

    /// Draw the depth-writing draws of one run of instances of `mesh`. With `only`, just the draws whose
    /// [`static_depth::dynamic`] equals it. Returns whether a dynamic draw was left out.
    fn draw_run<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'_>,
        binder: &mut Binder<'a>,
        (renderer, frame): (&'a Renderer, &FrameParams),
        (first, end, mirrored): (usize, usize, bool),
        mesh: &GpuMesh,
        only: Option<bool>,
    ) -> bool {
        let mut skipped = false;
        let mut bound_mesh = false;
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
            let dynamic = static_depth::dynamic(draw.shading);
            if only.is_some_and(|wanted| wanted != dynamic) {
                skipped |= dynamic;
                continue;
            }
            let pipeline = self.pipeline(renderer, frame, draw, mirrored);
            if binder.pipeline.is_none_or(|bound| !std::ptr::eq(bound, pipeline)) {
                pass.set_pipeline(pipeline);
                binder.pipeline = Some(pipeline);
            }
            let slot = draw.texture.map(|t| renderer.redirects.resolve(t.raw()));
            let Some(texture) = slot.and_then(|s| renderer.textures.get(s)).or(renderer.textures.get(0)) else {
                continue;
            };
            if binder.texture.is_none_or(|bound| !std::ptr::eq(bound, texture)) {
                pass.set_bind_group(1, texture, &[]);
                binder.texture = Some(texture);
            }
            if !bound_mesh {
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                bound_mesh = true;
            }
            pass.draw_indexed(
                draw.first_index..draw.first_index + draw.index_count,
                draw.base_vertex,
                first as u32..end as u32,
            );
        }
        skipped
    }

    fn pipeline(
        &self,
        renderer: &Renderer,
        frame: &FrameParams,
        draw: &DrawRange,
        mirrored: bool,
    ) -> &wgpu::RenderPipeline {
        let original = renderer.experiments.original_world_materials;
        match draw.blend {
            BlendMode::AlphaTest if renderer.reflective_surface(draw.shading) => {
                &self.opaque[Face::of(draw, mirrored).seen(frame.mirrored_view()).index()]
            }
            BlendMode::AlphaTest if matches!(draw.shading, Shading::Glossy(_)) => &self.glossy_cutout,
            BlendMode::AlphaTest if original && draw.shading == Shading::Tree => &self.tree_cutout,
            BlendMode::AlphaTest if original && matches!(draw.shading, Shading::WorldWindow(_)) => &self.window_cutout,
            BlendMode::AlphaTest if original && draw.shading == Shading::WorldPrelit => &self.prelit_cutout,
            BlendMode::AlphaTest
                if matches!(
                    draw.shading,
                    Shading::World | Shading::WorldDistant | Shading::WorldNoFog | Shading::WorldPrelit
                ) =>
            {
                &self.world_cutout
            }
            BlendMode::AlphaTest => &self.cutout,
            _ => &self.opaque[Face::of(draw, mirrored).seen(frame.mirrored_view()).index()],
        }
    }
}

/// The pipeline and texture bound last in a prepass, so repeated ones are not set again.
#[derive(Default)]
struct Binder<'a> {
    pipeline: Option<&'a wgpu::RenderPipeline>,
    texture: Option<&'a wgpu::BindGroup>,
}
