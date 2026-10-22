//! Drawing a frame: the scene straight into the surface, or into an offscreen image that the
//! post-process chain then writes to the surface, then the UI over it.

use super::Renderer;
use super::effects::SoftDraw;
use super::output::Output;
use super::pipelines::{BLEND_ORDER, SHADINGS};
use super::post::PassContext;
use super::resources::Globals;
use super::targets::SceneTargets;
use crate::{DrawRange, FrameParams, Instance, RenderError, Shading};

impl Renderer {
    /// Draw `instances` and present. Returns `Ok(false)` when the frame was skipped
    /// (window minimised or the surface had to be reconfigured). A headless renderer draws into its
    /// output texture and always returns `Ok(true)`.
    ///
    /// Instances of the same mesh should be adjacent: each run becomes one instanced draw per range.
    pub fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<bool, RenderError> {
        let _trace = blackbox_trace::scope("Renderer: frame");
        self.refresh_display(false);
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            self.presentation.render_start();
            self.prepare_generation(frame);
        }
        let acquire_started = std::time::Instant::now();
        let acquire_trace = blackbox_trace::scope("Wait: acquire surface");
        let (surface_texture, view) = match &self.output {
            #[cfg(all(windows, feature = "streamline"))]
            Output::Window { dx12: Some(native), .. } => (None, native.view()?),
            Output::Window { surface, config, .. } => {
                let acquired = surface.get_current_texture();
                blackbox_trace::global().counter(
                    "Surface acquisition",
                    &[("elapsed_ms", acquire_started.elapsed().as_secs_f64() * 1000.0)],
                );
                let outcome = match &acquired {
                    wgpu::CurrentSurfaceTexture::Success(_) => "success",
                    wgpu::CurrentSurfaceTexture::Suboptimal(_) => "suboptimal",
                    wgpu::CurrentSurfaceTexture::Timeout => "timeout",
                    wgpu::CurrentSurfaceTexture::Occluded => "occluded",
                    wgpu::CurrentSurfaceTexture::Outdated => "outdated",
                    wgpu::CurrentSurfaceTexture::Lost => "lost",
                    wgpu::CurrentSurfaceTexture::Validation => "validation",
                };
                if outcome != "success" {
                    blackbox_trace::global().note(
                        "Surface acquisition outcome",
                        &format!(
                            "{outcome}; present_mode={:?}; {}x{}",
                            config.present_mode, config.width, config.height
                        ),
                    );
                }
                let texture = match acquired {
                    wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                    wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return Ok(false),
                    wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                        #[cfg(all(windows, feature = "streamline"))]
                        if self.dlss.is_some() {
                            self.presentation.suspend(&self.device);
                        }
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
        self.performance.acquire_wait_ms = acquire_started.elapsed().as_secs_f32() * 1000.0;
        drop(acquire_trace);
        let submit_started = std::time::Instant::now();
        self.submit_frame(None, &view, self.surface_size(), frame, instances);
        self.performance.cpu_render_ms = submit_started.elapsed().as_secs_f32() * 1000.0;
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            self.presentation.marker(3);
            self.presentation.marker(4);
        }
        let present_started = std::time::Instant::now();
        let _present_trace = blackbox_trace::scope("Wait: present / FG dispatch");
        #[cfg(all(windows, feature = "streamline"))]
        if let Output::Window { dx12: Some(native), config, .. } = &self.output {
            native.present(
                &self.device,
                &self.queue,
                matches!(config.present_mode, wgpu::PresentMode::AutoVsync | wgpu::PresentMode::Fifo),
            )?;
        }
        match surface_texture {
            Some(t) => self.queue.present(t),
            // Nothing presents a headless frame, so let the device retire finished work itself.
            None => drop(self.device.poll(wgpu::PollType::Poll)),
        }
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            self.presentation.marker(5);
            self.presentation.finish();
        }
        self.performance.present_wait_ms = present_started.elapsed().as_secs_f32() * 1000.0;
        self.performance.rendered_frames += 1;
        self.count_frame();
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
        let _trace = blackbox_trace::scope("Renderer CPU: post-processing / upscaling");
        let Some(scene) = targets.unwrap_or(&self.targets).offscreen() else { return };
        let ctx = PassContext {
            device: &self.device,
            queue: &self.queue,
            scene,
            output_size: size,
            temporal: self.temporal.current(),
        };
        if self.experiments.lighting_debug != blackbox_gfx::LightingDebug::Off
            && self.info.api != blackbox_gfx::GraphicsApi::Gl
        {
            self.post.encode_diagnostic(&ctx, encoder, (output, output.texture().format()));
            return;
        }
        self.post.encode(&ctx, encoder, (output, output.texture().format()));
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
        let _trace = blackbox_trace::scope("Renderer CPU: encode scene");
        let size = targets.unwrap_or(&self.targets).size();
        // The colour pass and camera prepass use the same instance order and transforms.
        self.instances.upload_instances(&self.device, &self.queue, instances);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 4);
        }
        let depth = targets.unwrap_or(&self.targets).depth_view().clone();
        self.encode_surfaces(encoder, size, frame, instances, &depth);
        self.encode_effect_motion(encoder, frame, instances);
        self.encode_contact(encoder, size, frame);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 5);
            timing.stamp(encoder, 2);
        }
        self.encode_shadows(encoder, frame, instances);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 3);
        }
        self.encode_reflections(encoder, true);
        self.encode_irradiance(encoder);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 6);
        }
        self.encode_ambient(encoder, frame);
        if let Some(timing) = &mut self.timing {
            timing.stamp(encoder, 7);
        }
        self.encode_scene_mode(encoder, targets, output, frame, instances, false, None);
        self.encode_motion(encoder, targets, instances);
    }

    /// Reflection views omit effects, AO and glossy draws (no reflective-object feedback). `reduced` views
    /// (the rear-view mirror) take no shadows or AO in the shader, but keep effects and glossy draws.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn encode_scene_mode(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        targets: Option<&SceneTargets>,
        output: &wgpu::TextureView,
        frame: &FrameParams,
        instances: &[Instance],
        reduced: bool,
        capture: Option<&mut super::reflections::CaptureView>,
    ) {
        let _trace = blackbox_trace::scope("Renderer CPU: color view");
        let reflection = capture.is_some();
        let screen = !reflection
            && !reduced
            && self.experiments.screen_reflections
            && self.screen_lighting_active(frame)
            && self.glossy.is_some();
        if !screen
            && !reflection
            && let Some(glossy) = &self.glossy
        {
            glossy.screen.disable(&self.queue);
        }
        let projected = capture.as_ref().is_some_and(|v| v.kind == super::reflections::CaptureKind::Projected);
        let lit_cube = capture.as_ref().is_some_and(|v| v.kind == super::reflections::CaptureKind::LitCube);
        let [r, g, b] = frame.clear_color;
        let [fog_start, fog_end] = frame.fog_range();
        // A reflection, and a capture while a temporal upscaler is chosen, are drawn at their own size with no
        // upscale after, so the bias that matches texel density to the output would only sharpen them too much.
        let mip_bias = match reflection || (targets.is_some() && self.post.has_temporal()) {
            true => 0.0,
            false => self.upscale.texture_lod_bias,
        };
        let (scatter_rayleigh, scatter_mie, to_sun) = match frame.scattering {
            Some(s) => (
                [s.rayleigh[0], s.rayleigh[1], s.rayleigh[2], 1.0],
                [s.mie[0], s.mie[1], s.mie[2], s.g],
                s.to_sun.normalize_or_zero().extend(s.distance_scale).to_array(),
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
            view_proj: match reflection {
                true => frame.view_proj(),
                false => self.world_to_clip(frame),
            }
            .to_cols_array_2d(),
            // w is reserved for the HDR extension; reflection contracts stay SDR.
            camera_pos: frame
                .camera_position
                .extend(match !reflection && self.post.display.is_some() {
                    true => 2.0,
                    false => 1.0,
                })
                .to_array(),
            light_dir: frame
                .light_dir
                .normalize_or_zero()
                .extend(u8::from(self.experiments.original_world_materials || projected) as f32)
                .to_array(),
            fog_color: [r, g, b, frame.surface_light.map_or(1.0, |s| s.ambient_alpha)],
            fog_range: [
                fog_start,
                fog_end,
                mip_bias,
                match (projected, reflection || reduced) {
                    _ if lit_cube => 3.0,
                    (true, _) => 2.0,
                    (_, true) => 1.0,
                    _ => 0.0,
                },
            ],
            scatter_rayleigh,
            scatter_mie,
            to_sun,
            surface_ambient,
            surface_diffuse: [
                surface_diffuse[0],
                surface_diffuse[1],
                surface_diffuse[2],
                frame.scattering.map_or(1.0, |s| s.in_scatter_scale),
            ],
            camera_right: [frame.view.x_axis.x, frame.view.y_axis.x, frame.view.z_axis.x, 0.0],
        };
        let (uniform, bind_group, instance_buffer) = match capture {
            Some(view) => {
                view.instances.upload_instances(&self.device, &self.queue, instances);
                (&view.globals, &view.bind_group, &view.instances.buffer)
            }
            None => (&self.shared.bindings.globals, &self.shared.bindings.globals_bind_group, &self.instances.buffer),
        };
        self.queue.write_buffer(uniform, 0, bytemuck::bytes_of(&globals));
        let bind_group = bind_group.clone();
        let instance_buffer = instance_buffer.clone();

        let targets = targets.unwrap_or(&self.targets);
        let target_view = targets.offscreen().map_or(output, |offscreen| &offscreen.color_view).clone();
        let target = &target_view;
        let depth_view = targets.depth_view().clone();
        let depth = &depth_view;
        let prepared_depth = !reflection && self.surfaces.as_ref().is_some_and(|s| s.targets.depth == *depth);
        let format = targets.color_format();
        self.pipelines.use_format(&self.device, format, self.glossy.is_some());
        self.effects.use_format(&self.device, format);
        if let Some(sky) = &mut self.sky {
            sky.prepare(&self.device, &self.queue, format, self.pipeline_cache.as_ref());
        }
        if self.experiments.original_world_materials
            && !projected
            && let Some(normal) = &mut self.normal
        {
            normal.prepare(&self.device, &self.queue, format, frame, self.pipeline_cache.as_ref());
        }
        if (self.experiments.original_world_materials || projected)
            && let Some(window) = &mut self.window
        {
            window.prepare(&self.device, format, self.pipeline_cache.as_ref());
        }
        self.draw_list.build(instances, self.draw_revision, &self.meshes, &self.redirects, &self.textures);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(match projected {
                        true => wgpu::Color::BLACK,
                        false => wgpu::Color { r: r.into(), g: g.into(), b: b.into(), a: 1.0 },
                    }),
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
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(1, instance_buffer.slice(..));
        if let Some(sky) = &self.sky {
            sky.draw(&mut pass, format, &self.meshes, self.draw_list.sky());
        }

        // Every instance run and draw was sorted into its pipeline's bucket up front. A draw is only skipped
        // here when this view's rules (projected, reflection, original materials) exclude it.

        let original = self.experiments.original_world_materials;
        for (mode_index, mode) in BLEND_ORDER.into_iter().enumerate() {
            if original
                && !projected
                && let Some(normal) = &self.normal
            {
                normal.draw(
                    &mut pass,
                    format,
                    mode,
                    &self.meshes,
                    self.draw_list.normal(mode),
                    &self.shared.shadows.fallback,
                );
            }
            if (original || projected)
                && let Some(window) = &self.window
            {
                window.draw(
                    &mut pass,
                    format,
                    mode,
                    &self.meshes,
                    self.draw_list.window(mode),
                    &self.shared.shadows.fallback,
                );
            }
            for (shading_index, shading) in SHADINGS.into_iter().enumerate() {
                if ((screen && matches!(mode, blackbox_gfx::BlendMode::Opaque | blackbox_gfx::BlendMode::AlphaTest))
                    || (reflection && !projected))
                    && matches!(shading, Shading::Glossy(_))
                {
                    continue;
                }
                let Some(pipeline) = self.pipelines.get(mode, shading, prepared_depth) else { continue };
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
                let slot = mode_index * SHADINGS.len() + shading_index;
                let mut bound_mesh = None;
                let mut bound_texture = None;
                for &cmd in self.draw_list.bucket(slot) {
                    let Some(mesh) = self.meshes.get(cmd.mesh) else { continue };
                    let Some(d) = mesh.draws.get(cmd.draw as usize) else { continue };
                    if !drawn_here(d, shading, projected, original) {
                        continue;
                    }
                    if let (Shading::Glossy(material), Some(glossy)) = (d.shading, glossy)
                        && !glossy.bind_material(&mut pass, material)
                    {
                        continue;
                    }
                    // A projected normal draw shows the capture's diffuse texture when it has one.
                    let captured = match (projected, d.shading) {
                        (true, Shading::WorldNormal(handle)) => {
                            self.normal.as_ref().and_then(|n| n.capture_diffuse(handle))
                        }
                        _ => None,
                    };
                    let texture = match captured {
                        Some(t) => self.redirects.get(&t.raw()).copied().unwrap_or(t.raw()),
                        None => cmd.texture,
                    };
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
            }
            if screen && mode == blackbox_gfx::BlendMode::AlphaTest {
                drop(pass);
                self.encode_screen_glossy(encoder, target, depth, frame, instances);
                pass = super::glossy::screen_draw::resume(encoder, target, depth);
                pass.set_bind_group(0, &bind_group, &[]);
                pass.set_vertex_buffer(1, instance_buffer.slice(..));
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
            inverse_view_proj: self.world_to_clip(frame).inverse(),
        };
        self.effects.draw_soft(&self.device, &self.queue, encoder, &soft);
    }
}

/// Whether this view draws `d` in the pipeline `shading`. The bucket holds the draws that share a pipeline with
/// it; the rules here decide which of those (and which neighbours a view borrows) this view draws.
fn drawn_here(d: &DrawRange, shading: Shading, projected: bool, original: bool) -> bool {
    let own = d.shading.same_pipeline(shading) && !(projected && d.shading == Shading::World);
    let projected_borrow =
        projected && shading == Shading::WorldNoFog && matches!(d.shading, Shading::World | Shading::WorldNormal(_));
    let prelit_borrow = !projected
        && !original
        && shading == Shading::Prelit
        && matches!(d.shading, Shading::WorldNormal(_) | Shading::WorldWindow(_));
    own || projected_borrow || prelit_borrow
}
