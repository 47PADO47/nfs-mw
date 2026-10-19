//! Split the optional glossy pass after opaque scenery; reflection rays cannot sample the HUD.
use crate::gpu::{Renderer, pipelines::BLEND_ORDER};
use blackbox_gfx::{FrameParams, Instance, Projection, Shading};

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gpu) fn encode_screen_glossy(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        frame: &FrameParams,
        instances: &[Instance],
    ) {
        let _trace = blackbox_trace::scope("Renderer CPU: selective reflections");
        let Projection::PerspectiveInfiniteReverse { near, .. } = frame.projection else { return };
        let Some(glossy) = &mut self.glossy else { return };
        glossy.screen.capture(&self.device, &self.queue, encoder, target, depth, near);
        glossy.scene = super::scene_group(
            &self.device,
            &glossy.layouts,
            &glossy.rig,
            &glossy.environment,
            &glossy.faces,
            &glossy.sampler,
            &glossy.irradiance.buffer,
            &glossy.screen,
        );
        let mut pass = resume(encoder, target, depth);
        pass.set_bind_group(0, &self.shared.bindings.globals_bind_group, &[]);
        pass.set_vertex_buffer(1, self.instances.buffer.slice(..));
        for mode in BLEND_ORDER.into_iter().take(2) {
            let Some(pipeline) =
                self.pipelines.get(mode, Shading::Glossy(blackbox_gfx::GlossyMaterialHandle::from_raw(0)), false)
            else {
                continue;
            };
            pass.set_pipeline(pipeline);
            glossy.bind_scene(&mut pass);
            let mut first = 0;
            while first < instances.len() {
                let handle = instances[first].mesh;
                let end = first + instances[first..].iter().take_while(|i| i.mesh == handle).count();
                if let Some(mesh) = self.meshes.get(handle.raw()) {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                    for draw in &mesh.draws {
                        let Shading::Glossy(material) = draw.shading else { continue };
                        if draw.blend != mode || !glossy.bind_material(&mut pass, material) {
                            continue;
                        }
                        let slot = draw.texture.map(|t| self.redirects.get(&t.raw()).copied().unwrap_or(t.raw()));
                        let Some(texture) = slot.and_then(|s| self.textures.get(s)).or(self.textures.get(0)) else {
                            continue;
                        };
                        pass.set_bind_group(1, texture, &[]);
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
