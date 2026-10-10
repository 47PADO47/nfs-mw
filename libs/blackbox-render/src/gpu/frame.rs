//! Drawing a frame: the scene straight into the surface, or into an offscreen image that the
//! post-process chain then writes to the surface, then the UI over it.

use super::Renderer;
use super::pipelines::{BLEND_ORDER, SHADINGS};
use super::post::PassContext;
use super::resources::Globals;
use super::targets::SceneTargets;
use crate::{FrameParams, Instance, RenderError, Shading};

impl Renderer {
    /// Draw `instances` and present. Returns `Ok(false)` when the frame was skipped
    /// (window minimised or the surface had to be reconfigured).
    ///
    /// Instances of the same mesh should be adjacent: each run becomes one instanced draw per range.
    pub fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<bool, RenderError> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(false),
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(false);
            }
            #[allow(unreachable_patterns)]
            other => return Err(RenderError::Surface(format!("{other:?}"))),
        };
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        self.encode_scene(&mut encoder, None, &view, frame, instances);
        self.encode_post(&mut encoder, None, &view, self.surface_size());
        self.encode_ui(&mut encoder, &view, self.surface_size());
        self.queue.submit([encoder.finish()]);
        self.queue.present(surface_texture);
        Ok(true)
    }

    /// Record the post-process chain: the scene image of `targets` (default: the window's) to `output`,
    /// which is `size` pixels in the surface format. A scene drawn straight into `output` has no chain.
    pub(super) fn encode_post(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        targets: Option<&SceneTargets>,
        output: &wgpu::TextureView,
        size: (u32, u32),
    ) {
        let Some(scene) = targets.unwrap_or(&self.targets).offscreen() else { return };
        let ctx = PassContext { device: &self.device, queue: &self.queue, scene, output_size: size };
        self.post.encode(&ctx, encoder, (output, self.config.format));
    }

    /// Record the scene into the colour and depth of `targets` (default: the window's): the offscreen
    /// image, or `output` itself when the targets are direct.
    pub(super) fn encode_scene(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        targets: Option<&SceneTargets>,
        output: &wgpu::TextureView,
        frame: &FrameParams,
        instances: &[Instance],
    ) {
        let [r, g, b] = frame.clear_color;
        let globals = Globals {
            view_proj: frame.view_proj.to_cols_array_2d(),
            camera_pos: frame.camera_position.extend(1.0).to_array(),
            light_dir: frame.light_dir.normalize_or_zero().extend(0.0).to_array(),
            fog_color: [r, g, b, 1.0],
            fog_range: [frame.fog_start, frame.fog_end, self.upscale.texture_lod_bias, 0.0],
        };
        self.queue.write_buffer(&self.shared.globals, 0, bytemuck::bytes_of(&globals));
        let matrices: Vec<[f32; 16]> = instances.iter().map(|i| i.transform.to_cols_array()).collect();
        self.instances.upload(&self.device, &self.queue, &matrices);

        let targets = targets.unwrap_or(&self.targets);
        let target = targets.offscreen().map_or(output, |offscreen| &offscreen.color_view);
        let depth = targets.depth_view();
        let format = targets.color_format();
        self.pipelines.use_format(&self.device, format, self.glossy.is_some());
        self.effects.use_format(&self.device, format);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: r.into(), g: g.into(), b: b.into(), a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(0.0), store: wgpu::StoreOp::Store }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.shared.globals_bind_group, &[]);
        pass.set_vertex_buffer(1, self.instances.buffer.slice(..));

        for (mode, shading) in BLEND_ORDER.iter().flat_map(|&b| SHADINGS.iter().map(move |&s| (b, s))) {
            let Some(pipeline) = self.pipelines.get(mode, shading) else { continue };
            pass.set_pipeline(pipeline);
            let glossy = match shading {
                Shading::Glossy(_) => match &self.glossy {
                    Some(glossy) => Some(glossy),
                    None => continue,
                },
                _ => None,
            };
            if let Some(glossy) = glossy {
                glossy.bind_scene(&mut pass);
            }
            let mut start = 0;
            while start < instances.len() {
                let mesh_handle = instances[start].mesh;
                let end = start + instances[start..].iter().take_while(|i| i.mesh == mesh_handle).count();
                if let Some(mesh) = self.meshes.get(mesh_handle.raw()) {
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                    for d in mesh.draws.iter().filter(|d| d.blend == mode && d.shading.same_pipeline(shading)) {
                        if let (Shading::Glossy(material), Some(glossy)) = (d.shading, glossy)
                            && !glossy.bind_material(&mut pass, material)
                        {
                            continue;
                        }
                        let slot = d.texture.map(|t| self.redirects.get(&t.raw()).copied().unwrap_or(t.raw()));
                        let texture = slot.and_then(|s| self.textures.get(s)).or(self.textures.get(0));
                        if let Some(texture) = texture {
                            pass.set_bind_group(1, texture, &[]);
                            pass.draw_indexed(
                                d.first_index..d.first_index + d.index_count,
                                d.base_vertex,
                                start as u32..end as u32,
                            );
                        }
                    }
                }
                start = end;
            }
        }
        self.effects.draw(&mut pass);
        self.effects.textured.draw(&mut pass, &self.textures);
        drop(pass);
        self.effects.draw_soft((&self.device, &self.queue), encoder, (target, depth), &self.shared, frame);
    }
}
