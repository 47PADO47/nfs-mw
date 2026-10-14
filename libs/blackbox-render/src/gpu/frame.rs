//! Drawing a frame: the scene straight into the surface, or into an offscreen image that the
//! post-process chain then writes to the surface, then the UI over it.

use super::Renderer;
use super::effects::SoftDraw;
use super::output::Output;
use super::pipelines::{BLEND_ORDER, SHADINGS};
use super::post::PassContext;
use super::resources::Globals;
use super::targets::SceneTargets;
use crate::{FrameParams, Instance, RenderError, Shading};

impl Renderer {
    /// Draw `instances` and present. Returns `Ok(false)` when the frame was skipped
    /// (window minimised or the surface had to be reconfigured). A headless renderer draws into its
    /// output texture and always returns `Ok(true)`.
    ///
    /// Instances of the same mesh should be adjacent: each run becomes one instanced draw per range.
    pub fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<bool, RenderError> {
        let (surface_texture, view) = match &self.output {
            Output::Window { surface, config } => {
                let texture = match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(false),
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        surface.configure(&self.device, config);
                        return Ok(false);
                    }
                    #[allow(unreachable_patterns)]
                    other => return Err(RenderError::Surface(format!("{other:?}"))),
                };
                let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
                (Some(texture), view)
            }
            Output::Texture { texture, .. } => (None, texture.create_view(&wgpu::TextureViewDescriptor::default())),
        };
        self.submit_frame(None, &view, self.surface_size(), frame, instances);
        match surface_texture {
            Some(t) => self.queue.present(t),
            // Nothing presents a headless frame, so let the device retire finished work itself.
            None => drop(self.device.poll(wgpu::PollType::Poll)),
        }
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
        if self.experiments.lighting_debug != blackbox_gfx::LightingDebug::Off
            && self.info.api != blackbox_gfx::GraphicsApi::Gl
        {
            self.post.encode_diagnostic(&ctx, encoder, (output, self.output.format()));
            return;
        }
        self.post.encode(&ctx, encoder, (output, self.output.format()));
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
        let size = targets.unwrap_or(&self.targets).size();
        // The colour pass and camera prepass use the same instance order and transforms.
        self.instances.upload_instances(&self.device, &self.queue, instances);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 4);
        }
        let depth = targets.unwrap_or(&self.targets).depth_view().clone();
        self.encode_surfaces(encoder, size, frame, instances, &depth);
        self.encode_contact(encoder, size, frame);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 5);
            timing.stamp(encoder, 2);
        }
        self.encode_shadows(encoder, frame, instances);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 3);
        }
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 6);
        }
        self.encode_ambient(encoder, frame);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 7);
        }
        self.encode_scene_mode(encoder, targets, output, frame, instances, None);
    }

    /// Reflection views omit effects, AO and glossy draws (no reflective-object feedback).
    pub(super) fn encode_scene_mode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        targets: Option<&SceneTargets>,
        output: &wgpu::TextureView,
        frame: &FrameParams,
        instances: &[Instance],
        capture: Option<&mut super::reflections::CaptureView>,
    ) {
        let reflection = capture.is_some();
        let [r, g, b] = frame.clear_color;
        let [fog_start, fog_end] = frame.fog_range();
        let mip_bias = match reflection {
            true => 0.0,
            false => self.upscale.texture_lod_bias,
        };
        let (scatter_rayleigh, scatter_mie, to_sun) = match frame.scattering {
            Some(s) => (
                [s.rayleigh[0], s.rayleigh[1], s.rayleigh[2], 1.0],
                [s.mie[0], s.mie[1], s.mie[2], s.g],
                s.to_sun.normalize_or_zero().extend(0.0).to_array(),
            ),
            None => ([0.0; 4], [0.0; 4], [0.0; 4]),
        };
        let (surface_ambient, surface_diffuse) = match frame.surface_light {
            Some(l) => {
                ([l.ambient[0], l.ambient[1], l.ambient[2], 1.0], [l.diffuse[0], l.diffuse[1], l.diffuse[2], 0.0])
            }
            None => ([0.0; 4], [0.0; 4]),
        };
        let globals = Globals {
            view_proj: frame.view_proj().to_cols_array_2d(),
            camera_pos: frame.camera_position.extend(1.0).to_array(),
            light_dir: frame.light_dir.normalize_or_zero().extend(0.0).to_array(),
            fog_color: [r, g, b, 1.0],
            fog_range: [fog_start, fog_end, mip_bias, u8::from(reflection) as f32],
            scatter_rayleigh,
            scatter_mie,
            to_sun,
            surface_ambient,
            surface_diffuse,
        };
        let (uniform, bind_group, instance_buffer) = match capture {
            Some(view) => {
                view.instances.upload_instances(&self.device, &self.queue, instances);
                (&view.globals, &view.bind_group, &view.instances.buffer)
            }
            None => (&self.shared.bindings.globals, &self.shared.bindings.globals_bind_group, &self.instances.buffer),
        };
        self.queue.write_buffer(uniform, 0, bytemuck::bytes_of(&globals));

        let targets = targets.unwrap_or(&self.targets);
        let target = targets.offscreen().map_or(output, |offscreen| &offscreen.color_view);
        let depth = targets.depth_view();
        let prepared_depth = !reflection && self.surfaces.as_ref().is_some_and(|s| s.targets.depth == *depth);
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
                depth_ops: Some(wgpu::Operations {
                    load: match prepared_depth {
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
        });
        pass.set_bind_group(0, bind_group, &[]);
        pass.set_vertex_buffer(1, instance_buffer.slice(..));

        for (mode, shading) in BLEND_ORDER.iter().flat_map(|&b| SHADINGS.iter().map(move |&s| (b, s))) {
            if reflection && matches!(shading, Shading::Glossy(_)) {
                continue;
            }
            let Some(pipeline) = self.pipelines.get(mode, shading) else { continue };
            pass.set_pipeline(pipeline);
            if !matches!(shading, Shading::Glossy(_)) {
                pass.set_bind_group(2, &self.shared.shadows.fallback, &[]);
            }
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
                    let mut draws =
                        mesh.draws.iter().filter(|d| d.blend == mode && d.shading.same_pipeline(shading)).peekable();
                    if draws.peek().is_none() {
                        start = end;
                        continue;
                    }
                    pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                    pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint16);
                    for d in draws {
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
        if reflection
            || (self.experiments.lighting_debug != blackbox_gfx::LightingDebug::Off
                && self.info.api != blackbox_gfx::GraphicsApi::Gl)
        {
            return;
        }
        self.effects.draw(&mut pass);
        self.effects.draw_textured(&mut pass, |t| self.textures.get(t.raw()));
        drop(pass);
        let soft = SoftDraw {
            target,
            depth,
            globals: &self.shared.bindings.globals_bind_group,
            inverse_view_proj: frame.view_proj().inverse(),
        };
        self.effects.draw_soft(&self.device, &self.queue, encoder, &soft);
    }
}
