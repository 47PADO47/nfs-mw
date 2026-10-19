//! One material-identical raster path for uncached, static and dynamic casters.
use super::{Shadows, casts};
use crate::gpu::Renderer;
use blackbox_gfx::{BlendMode, ExperimentStats, Instance, Shading};
impl Shadows {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_selected(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        cascade: usize,
        target: &wgpu::TextureView,
        size: f32,
        load: wgpu::LoadOp<f32>,
        buffer: &wgpu::Buffer,
        selected: &[usize],
        casters: &[Instance],
        stats: &mut ExperimentStats,
    ) {
        let settings = renderer.experiments;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("spatial sun cascade"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target,
                depth_ops: Some(wgpu::Operations { load, store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_viewport(0.0, 0.0, size, size, 0.0, 1.0);
        pass.set_scissor_rect(0, 0, size as u32, size as u32);
        pass.set_bind_group(0, &self.groups[cascade], &[]);
        pass.set_vertex_buffer(1, buffer.slice(..));
        let mut bound_pipeline: Option<&wgpu::RenderPipeline> = None;
        let mut bound_texture: Option<&wgpu::BindGroup> = None;
        let mut first = 0;
        while first < selected.len() {
            let mesh_handle = casters[selected[first]].mesh;
            let end = first + selected[first..].iter().take_while(|&&index| casters[index].mesh == mesh_handle).count();
            let mesh = renderer.meshes.get(mesh_handle.raw()).expect("selected resident mesh");
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
            for draw in &mesh.draws {
                if !casts(draw.blend, draw.shading) {
                    continue;
                }
                let pipeline = match draw.blend {
                    BlendMode::AlphaTest
                        if settings.original_world_materials
                            && matches!(draw.shading, Shading::WorldNormal(handle)
                                if renderer.normal.as_ref().is_some_and(|n| n.reflective_material(handle))) =>
                    {
                        &self.opaque
                    }
                    BlendMode::AlphaTest
                        if settings.original_world_materials
                            && matches!(draw.shading, Shading::Tree | Shading::WorldWindow(_)) =>
                    {
                        &self.tree_cutout
                    }
                    BlendMode::AlphaTest
                        if settings.original_world_materials && draw.shading == Shading::WorldPrelit =>
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
                let slot = draw.texture.map(|t| renderer.redirects.get(&t.raw()).copied().unwrap_or(t.raw()));
                let texture = slot.and_then(|s| renderer.textures.get(s)).or(renderer.textures.get(0));
                let Some(texture) = texture else { continue };
                if bound_texture.is_none_or(|bound| !std::ptr::eq(bound, texture)) {
                    pass.set_bind_group(1, texture, &[]);
                    bound_texture = Some(texture);
                }
                pass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    draw.base_vertex,
                    first as u32..end as u32,
                );
                stats.shadow_draws += 1;
                stats.shadow_instances += (end - first) as u32;
            }
            first = end;
        }
    }
}
