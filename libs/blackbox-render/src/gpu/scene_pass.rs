//! A view's colour pass, recorded from what was prepared for it ([`PreparedView`]) and from renderer state the pass
//! only reads ([`SceneDraw`]). Everything a view writes or builds (its uniforms, its instance upload, the pipelines
//! and materials of its format, its draw lists) is done before, by `Renderer::prepare_view`, so the passes of several
//! views can be recorded at the same time on worker threads (the reflection cube's faces).

use super::Renderer;
use super::cull::Face;
use super::draw_list::{Cmd, DrawList, drawn_here};
use super::glossy::Glossy;
use super::meshes::GpuMesh;
use super::normal::Normal;
use super::pipelines::{BLEND_ORDER, Pipelines, SHADINGS};
use super::redirects::Redirects;
use super::sky::Sky;
use super::slots::Slots;
use super::window::Window;
use crate::{BlendMode, Shading};

/// One view's colour pass, prepared: where it draws, what it binds and which draws it keeps.
pub(super) struct PreparedView {
    /// The colour image the pass draws into.
    pub(super) target: wgpu::TextureView,
    pub(super) depth: wgpu::TextureView,
    pub(super) clear: wgpu::Color,
    /// The view's globals group (group 0) and instance buffer.
    pub(super) globals: wgpu::BindGroup,
    pub(super) instances: wgpu::Buffer,
    /// Where the per-instance data of the non-glossy pipelines starts in `instances`. The glossy pipelines read
    /// matrices from its start.
    pub(super) slots_at: u64,
    /// The instance store's bind group, when the renderer has one.
    pub(super) store: Option<wgpu::BindGroup>,
    pub(super) format: wgpu::TextureFormat,
    /// The camera prepass filled the depth: the pass loads it and takes the depth-prepared pipelines.
    pub(super) prepared_depth: bool,
    /// Whether the view's camera mirrors, which swaps the faces a single-sided draw culls.
    pub(super) mirrored_view: bool,
    pub(super) reflection: bool,
    pub(super) projected: bool,
    pub(super) planar: bool,
    /// Screen-space reflections replace the opaque and alpha-tested glossy draws.
    pub(super) screen: bool,
}

/// What a view's colour pass reads of the renderer. Nothing here is written while a pass records, so several passes
/// may share it across threads.
#[derive(Clone, Copy)]
pub(super) struct SceneDraw<'a> {
    pub(super) meshes: &'a Slots<GpuMesh>,
    pub(super) textures: &'a Slots<wgpu::BindGroup>,
    pub(super) redirects: &'a Redirects,
    pub(super) pipelines: &'a Pipelines,
    pub(super) sky: Option<&'a Sky>,
    pub(super) normal: Option<&'a Normal>,
    pub(super) window: Option<&'a Window>,
    pub(super) glossy: Option<&'a Glossy>,
    /// The shadow group (group 2) of the non-glossy draws.
    pub(super) shadows: &'a wgpu::BindGroup,
    /// The world is drawn with its authored materials (normal-map scenery and windows).
    pub(super) original: bool,
}

impl Renderer {
    /// The state every colour pass reads, borrowed from the renderer.
    pub(super) fn scene_draw(&self) -> SceneDraw<'_> {
        SceneDraw {
            meshes: &self.meshes,
            textures: &self.textures,
            redirects: &self.redirects,
            pipelines: &self.pipelines,
            sky: self.sky.as_ref(),
            normal: self.normal.as_ref(),
            window: self.window.as_ref(),
            glossy: self.glossy.as_ref(),
            shadows: &self.shared.shadows.fallback,
            original: self.experiments.original_world_materials,
        }
    }
}

/// Begin `view`'s colour pass in `encoder`: clear its colour, and clear its depth unless the prepass filled it.
pub(super) fn begin_pass<'e>(encoder: &'e mut wgpu::CommandEncoder, view: &PreparedView) -> wgpu::RenderPass<'e> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("scene"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view.target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(view.clear), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &view.depth,
            depth_ops: Some(wgpu::Operations {
                load: match view.prepared_depth {
                    true => wgpu::LoadOp::Load,
                    false => wgpu::LoadOp::Clear(0.0),
                },
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

impl<'a> SceneDraw<'a> {
    /// Record the whole colour pass of a view that draws no effects and no material reflections in between (a
    /// reflection view), with the draws of `list`.
    pub(super) fn record(&self, encoder: &mut wgpu::CommandEncoder, view: &PreparedView, list: &DrawList) {
        let mut pass = begin_pass(encoder, view);
        self.begin(&mut pass, view, list);
        for (mode_index, mode) in BLEND_ORDER.into_iter().enumerate() {
            self.blend_mode(&mut pass, view, list, (mode_index, mode));
        }
    }

    /// Bind the view's globals and instances, and draw its sky layers.
    pub(super) fn begin<'p>(&self, pass: &mut wgpu::RenderPass<'p>, view: &PreparedView, list: &DrawList)
    where
        'a: 'p,
    {
        pass.set_bind_group(0, &view.globals, &[]);
        pass.set_vertex_buffer(1, view.instances.slice(view.slots_at..));
        let Some(sky) = self.sky else { return };
        sky.draw(pass, view.format, self.meshes, list.sky(), view.store.as_ref());
    }

    /// Draw the view's normal-map, window and colour draws of blend mode `mode`, number `mode_index` in
    /// `BLEND_ORDER`.
    pub(super) fn blend_mode<'p>(
        &self,
        pass: &mut wgpu::RenderPass<'p>,
        view: &PreparedView,
        list: &DrawList,
        (mode_index, mode): (usize, BlendMode),
    ) where
        'a: 'p,
    {
        let (original, projected, mirrored_view) = (self.original, view.projected, view.mirrored_view);
        if original
            && !projected
            && let Some(normal) = self.normal
        {
            let list = list.normal(mode_index);
            normal.draw(
                pass,
                view.format,
                (mode, mirrored_view, false),
                self.meshes,
                list,
                self.shadows,
                view.store.as_ref(),
            );
        }
        if (original || projected)
            && let Some(window) = self.window
        {
            let list = list.window(mode_index);
            let layer = (mode, mirrored_view);
            window.draw(pass, view.format, layer, self.meshes, list, self.shadows, view.store.as_ref());
        }
        for (shading_index, shading) in SHADINGS.into_iter().enumerate() {
            let bucket = list.bucket(mode_index * SHADINGS.len() + shading_index);
            self.bucket(pass, view, bucket, (mode, shading));
        }
    }

    /// Draw one pipeline's bucket of the view.
    fn bucket<'p>(
        &self,
        pass: &mut wgpu::RenderPass<'p>,
        view: &PreparedView,
        bucket: &[Cmd],
        (mode, shading): (BlendMode, Shading),
    ) where
        'a: 'p,
    {
        if bucket.is_empty() {
            return;
        }
        let (projected, original, mirrored_view) = (view.projected, self.original, view.mirrored_view);
        if ((view.screen && matches!(mode, BlendMode::Opaque | BlendMode::AlphaTest))
            || (view.reflection && !projected && !view.planar))
            && matches!(shading, Shading::Glossy(_))
        {
            return;
        }
        let prepared_depth = view.prepared_depth;
        let Some(pipeline) = self.pipelines.get(mode, shading, prepared_depth, Face::Both) else { return };
        pass.set_pipeline(pipeline);
        let mut bound_face = Face::Both;
        let glossy_pipeline = matches!(shading, Shading::Glossy(_));
        if !glossy_pipeline {
            pass.set_bind_group(2, self.shadows, &[]);
        }
        if !glossy_pipeline && let Some(store) = &view.store {
            pass.set_bind_group(3, store, &[]);
        }
        let glossy = match shading {
            Shading::Glossy(_) => match self.glossy {
                Some(glossy) => Some(glossy),
                None => return,
            },
            _ => None,
        };
        if let Some(glossy) = glossy {
            glossy.bind_scene(pass);
        }
        // The glossy shader reads matrices, which the view's buffer holds from its start (restored below).
        if glossy_pipeline && view.slots_at != 0 {
            pass.set_vertex_buffer(1, view.instances.slice(..));
        }
        let mut bound_mesh = None;
        let mut bound_texture = None;
        for &cmd in bucket {
            let Some(mesh) = self.meshes.get(cmd.mesh) else { continue };
            let Some(d) = mesh.draws.get(cmd.draw as usize) else { continue };
            if !drawn_here(d, shading, projected, original) {
                continue;
            }
            if let (Shading::Glossy(material), Some(glossy)) = (d.shading, glossy)
                && !glossy.bind_material(pass, material)
            {
                continue;
            }
            // A projected normal draw shows the capture's diffuse texture when it has one.
            let captured = match (projected, d.shading) {
                (true, Shading::WorldNormal(handle)) => self.normal.and_then(|n| n.capture_diffuse(handle)),
                _ => None,
            };
            let texture = match captured {
                Some(t) => self.redirects.resolve(t.raw()),
                None => cmd.texture,
            };
            let face = cmd.face.seen(mirrored_view);
            if face != bound_face
                && let Some(p) = self.pipelines.get(mode, shading, prepared_depth, face)
            {
                pass.set_pipeline(p);
                bound_face = face;
            }
            if bound_mesh != Some(cmd.mesh) {
                pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                bound_mesh = Some(cmd.mesh);
            }
            if bound_texture != Some(texture) {
                let Some(bind) = self.textures.get(texture).or(self.textures.get(0)) else { continue };
                pass.set_bind_group(1, bind, &[]);
                bound_texture = Some(texture);
            }
            pass.draw_indexed(
                d.first_index..d.first_index + d.index_count,
                d.base_vertex,
                cmd.instances.0..cmd.instances.1,
            );
        }
        if glossy_pipeline && view.slots_at != 0 {
            pass.set_vertex_buffer(1, view.instances.slice(view.slots_at..));
        }
    }
}
