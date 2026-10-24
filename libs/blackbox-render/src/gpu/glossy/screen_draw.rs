//! Split the optional glossy pass after opaque scenery; reflection rays cannot sample the HUD.
use super::{Glossy, screen::SceneKey};
use crate::gpu::{
    Renderer,
    cull::Face,
    pipelines::{BLEND_ORDER, SHADINGS},
};
use blackbox_gfx::{FrameParams, Projection, Shading};

/// The glossy shading's position among `SHADINGS`, which is its bucket within a blend mode's buckets.
const GLOSSY_SHADING: usize = SHADINGS.len() - 1;
const _: () = assert!(matches!(SHADINGS[GLOSSY_SHADING], Shading::Glossy(_)));

impl Renderer {
    /// Draw the opaque and cutout glossy draws of the view built into `self.draw_list`, over the world colour
    /// captured just before.
    pub(in crate::gpu) fn encode_screen_glossy(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        frame: &FrameParams,
    ) {
        let _trace = blackbox_trace::scope("Renderer CPU: selective reflections");
        let Projection::PerspectiveInfiniteReverse { near, .. } = frame.projection else { return };
        let Some(glossy) = &mut self.glossy else { return };
        glossy.screen.capture(&self.device, encoder, target, depth);
        glossy.screen.enable(&self.queue, [1.0, near, 50.0, 0.0]);
        glossy.scene = screen_scene(glossy, &self.device);
        let mut pass = resume(encoder, target, depth);
        pass.set_bind_group(0, &self.shared.bindings.globals_bind_group, &[]);
        pass.set_vertex_buffer(1, self.instances.buffer.slice(..));
        let shading = Shading::Glossy(blackbox_gfx::GlossyMaterialHandle::from_raw(0));
        // The colour pass's own glossy buckets: the same draws in the same order the instance walk made.
        for (mode_index, mode) in BLEND_ORDER.into_iter().take(2).enumerate() {
            let bucket = self.draw_list.bucket(mode_index * SHADINGS.len() + GLOSSY_SHADING);
            if bucket.is_empty() {
                continue;
            }
            let Some(pipeline) = self.pipelines.get(mode, shading, false, Face::Both) else { continue };
            pass.set_pipeline(pipeline);
            let mut bound_face = Face::Both;
            glossy.bind_scene(&mut pass);
            let mut bound_mesh = None;
            let mut bound_material = None;
            let mut bound_texture = None;
            for &cmd in bucket {
                let Some(mesh) = self.meshes.get(cmd.mesh) else { continue };
                let Some(draw) = mesh.draws.get(cmd.draw as usize) else { continue };
                let Shading::Glossy(material) = draw.shading else { continue };
                if bound_material != Some(material.raw()) {
                    if !glossy.bind_material(&mut pass, material) {
                        continue;
                    }
                    bound_material = Some(material.raw());
                }
                let face = cmd.face.seen(frame.mirrored_view());
                if face != bound_face
                    && let Some(p) = self.pipelines.get(mode, shading, false, face)
                {
                    pass.set_pipeline(p);
                    bound_face = face;
                }
                if bound_texture != Some(cmd.texture) {
                    let Some(texture) = self.textures.get(cmd.texture).or(self.textures.get(0)) else { continue };
                    pass.set_bind_group(1, texture, &[]);
                    bound_texture = Some(cmd.texture);
                }
                if bound_mesh != Some(cmd.mesh) {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                    bound_mesh = Some(cmd.mesh);
                }
                pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    draw.base_vertex,
                    cmd.instances.0..cmd.instances.1,
                );
            }
        }
    }
}

/// The glossy scene group for the current environment and screen capture. The captured colour only changes with the
/// target's size, so the group is built again only when a view it binds was replaced.
fn screen_scene(glossy: &mut Glossy, device: &wgpu::Device) -> wgpu::BindGroup {
    let Glossy { layouts, rig, sampler, irradiance, probe, environment, faces, filtered, screen, .. } = glossy;
    let environment = &*environment;
    let (filtered, linear_filtered) =
        filtered.as_ref().map_or((environment, environment), |f| (&f.view, &f.linear_view));
    let key = SceneKey {
        environment: environment.clone(),
        faces: faces.clone(),
        filtered: filtered.clone(),
        linear_filtered: linear_filtered.clone(),
        color: screen.color.clone(),
        depth: screen.depth.clone(),
        probe: probe.depth.clone(),
    };
    screen.scene_group(key, |screen| {
        super::scene_group(
            device,
            layouts,
            rig,
            environment,
            faces,
            sampler,
            &irradiance.buffer,
            screen,
            filtered,
            linear_filtered,
            probe,
        )
    })
}
pub(in crate::gpu) fn resume<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    depth: &wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("glossy reflections / effects"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
