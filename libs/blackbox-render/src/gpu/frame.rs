//! Drawing a frame: the scene straight into the surface, or into an offscreen image that the
//! post-process chain then writes to the surface, then the UI over it. The surface image is acquired as late as the
//! frame allows (see `acquire.rs`).

use super::Renderer;
use super::acquire::{Acquired, FrameOutput, Recorded};
use super::draw_list::DrawList;
use super::draw_list::instance_view::upload_view;
use super::pipelines::BLEND_ORDER;
use super::post::PassContext;
use super::reflections::{CaptureKind, CaptureView};
use super::scene_pass::{PreparedView, begin_pass};
use super::targets::SceneTargets;
use crate::{FrameParams, Instance, RenderError};
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
        // The window's image is acquired once the work for the renderer's own images is recorded when the frame
        // allows it (see `acquire.rs`), else now, before anything is recorded.
        let late = self.acquires_late();
        let early = match late {
            true => None,
            false => match self.acquire_output()? {
                Some(acquired) => Some(acquired),
                None => {
                    self.flush_captures();
                    return Ok(false);
                }
            },
        };
        let submit_started = std::time::Instant::now();
        let output = match &early {
            Some(acquired) => FrameOutput::View(acquired.view()),
            None => FrameOutput::Surface,
        };
        let recorded = self.record_frame(None, output, self.surface_size(), frame, instances)?;
        let presented = match recorded {
            Recorded::Given => early,
            Recorded::Acquired(acquired) => Some(acquired),
            Recorded::Skipped => return Ok(false),
        };
        // A late acquire waited in the middle of the recording; that wait is not recording time.
        let waited = match late {
            true => self.performance.acquire_wait_ms,
            false => 0.0,
        };
        self.performance.cpu_render_ms = (submit_started.elapsed().as_secs_f32() * 1000.0 - waited).max(0.0);
        #[cfg(all(windows, feature = "streamline"))]
        if self.dlss.is_some() {
            self.presentation.marker(3);
            self.presentation.marker(4);
        }
        let present_started = std::time::Instant::now();
        let _present_trace = blackbox_trace::scope("Wait: present / FG dispatch");
        #[cfg(all(windows, feature = "streamline"))]
        if let super::output::Output::Window { dx12: Some(native), config, .. } = &self.output {
            native.present(
                &self.device,
                &self.queue,
                matches!(config.present_mode, wgpu::PresentMode::AutoVsync | wgpu::PresentMode::Fifo),
            )?;
        }
        match presented.and_then(Acquired::into_surface) {
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
        // Every encoder recorded with the instance store before this one has been submitted, so its slots can be
        // handed out again, except a queued capture's: the frame joins its epoch. The colour pass and camera prepass
        // use the same instance order and transforms.
        self.begin_store_epoch();
        self.upload_main_instances(instances);
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
        capture: Option<&mut CaptureView>,
    ) {
        let _trace = blackbox_trace::scope("Renderer CPU: color view");
        let view = self.prepare_view(encoder, (targets, output), frame, instances, reduced, capture, None);
        // A reflection view draws no effects and no material reflections: its pass is recorded whole.
        if view.reflection {
            self.scene_draw().record(encoder, &view, &self.draw_list);
            return;
        }
        let mut pass = begin_pass(encoder, &view);
        self.scene_draw().begin(&mut pass, &view, &self.draw_list);
        for (mode_index, mode) in BLEND_ORDER.into_iter().enumerate() {
            self.scene_draw().blend_mode(&mut pass, &view, &self.draw_list, (mode_index, mode));
            if mode == blackbox_gfx::BlendMode::AlphaTest
                && (view.screen || self.road_reflections_active(frame, reduced))
            {
                drop(pass);
                // The screen capture switches the glossy control on.
                self.screen_control_off &= !view.screen;
                self.encode_material_reflections(
                    encoder,
                    &view.target,
                    &view.depth,
                    frame,
                    instances,
                    (view.screen, reduced),
                );
                pass = super::glossy::screen_draw::resume(encoder, &view.target, &view.depth);
                pass.set_bind_group(0, &view.globals, &[]);
                pass.set_vertex_buffer(1, view.instances.slice(view.slots_at..));
            }
        }
        if self.experiments.lighting_debug != blackbox_gfx::LightingDebug::Off
            && self.info.api != blackbox_gfx::GraphicsApi::Gl
        {
            return;
        }
        self.effects.draw(&mut pass);
        self.effects.draw_textured(&mut pass, |t| self.textures.get(t.raw()));
        drop(pass);
        self.encode_soft_effects(encoder, &view.target, &view.depth, frame);
    }

    /// Prepare one view's colour pass: write its globals and upload its instances, build the pipelines and materials
    /// of its format, and sort its draws into `list` (default: the renderer's own lists). The view's scene is drawn
    /// into `targets` (default: the window's): the offscreen image, or `output` itself when the targets are direct.
    /// Everything the pass writes or builds is done here, so recording it only reads shared state (`scene_pass.rs`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_view(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        (targets, output): (Option<&SceneTargets>, &wgpu::TextureView),
        frame: &FrameParams,
        instances: &[Instance],
        reduced: bool,
        capture: Option<&mut CaptureView>,
        list: Option<&mut DrawList>,
    ) -> PreparedView {
        let reflection = capture.is_some();
        let screen = !reflection
            && !reduced
            && self.experiments.screen_reflections
            && self.screen_lighting_active(frame)
            && self.glossy.is_some();
        if !screen
            && !reflection
            && !self.screen_control_off
            && let Some(glossy) = &self.glossy
        {
            glossy.screen.disable(&self.queue);
            self.screen_control_off = true;
        }
        let projected = capture.as_ref().is_some_and(|v| v.kind == CaptureKind::Projected);
        let planar = capture.as_ref().is_some_and(|v| v.kind == CaptureKind::Planar);
        let globals = self.scene_globals(frame, capture.as_deref(), reduced, targets.is_some());
        let [r, g, b] = frame.clear_color;
        // `slots_at`: where the per-instance data of the non-glossy pipelines starts in the view's buffer. The glossy
        // pipelines read matrices from its start.
        let (uniform, bind_group, instance_buffer, slots_at) = match capture {
            Some(view) => {
                let store = &mut self.draw_list.store;
                let at = upload_view(store, &self.device, &self.queue, &self.meshes, &mut view.instances, instances);
                (&view.globals, &view.bind_group, &view.instances.buffer, at)
            }
            None => {
                self.ensure_main_instances(instances);
                let shared = &self.shared.bindings;
                (
                    &shared.globals,
                    &shared.globals_bind_group,
                    &self.instances.buffer,
                    self.draw_list.store.main_offset(),
                )
            }
        };
        self.queue.write_buffer(uniform, 0, bytemuck::bytes_of(&globals));
        let bind_group = bind_group.clone();
        let instance_buffer = instance_buffer.clone();
        let store = self.draw_list.store.bind_group().cloned();

        let targets = targets.unwrap_or(&self.targets);
        let target = targets.offscreen().map_or(output, |offscreen| &offscreen.color_view).clone();
        let depth = targets.depth_view().clone();
        let prepared_depth = !reflection && self.surfaces.as_ref().is_some_and(|s| s.targets.depth == depth);
        let format = targets.color_format();
        // The projected capture shares the main camera but keeps the opposite faces.
        let (culled, mirrored_view) = (self.pipelines.culled(), frame.mirrored_view() ^ projected);
        self.pipelines.use_format(&self.device, format, self.glossy.is_some());
        self.effects.use_format(&self.device, format);
        if let Some(sky) = &mut self.sky {
            sky.prepare(&self.device, &self.queue, format, self.pipeline_cache.as_ref());
            sky.prepare_clouds(&self.device, encoder, &self.shared.bindings.globals_layout, &bind_group, &globals);
        }
        if self.experiments.original_world_materials && !projected {
            self.prepare_normal(frame, format, culled);
        }
        if (self.experiments.original_world_materials || projected)
            && let Some(window) = &mut self.window
        {
            window.prepare(&self.device, (format, culled), self.pipeline_cache.as_ref());
        }
        match list {
            Some(list) => list.build(instances, &self.meshes, &self.redirects, &self.textures),
            None => self.draw_list.build(instances, &self.meshes, &self.redirects, &self.textures),
        }
        PreparedView {
            target,
            depth,
            clear: match (planar, projected) {
                (true, _) => wgpu::Color::TRANSPARENT,
                (_, true) => wgpu::Color::BLACK,
                _ => wgpu::Color { r: r.into(), g: g.into(), b: b.into(), a: 1.0 },
            },
            globals: bind_group,
            instances: instance_buffer,
            slots_at,
            store,
            format,
            prepared_depth,
            mirrored_view,
            reflection,
            projected,
            planar,
            screen,
        }
    }
}
