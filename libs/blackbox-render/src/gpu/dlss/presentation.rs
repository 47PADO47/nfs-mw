//! Presentation resources stay alive through Streamline's asynchronous present work.
use super::super::{Renderer, output::Output};
use super::{composition::Composite, native};
use ash::vk::Handle;
use blackbox_gfx::{LatencySettings, LatencyStatus, Projection};

unsafe extern "C" {
    fn bb_sl_latency_caps(physical: *mut std::ffi::c_void, caps: *mut u32);
    fn bb_sl_begin(mode: u32, limit_us: u32) -> i32;
    fn bb_sl_render_start();
    fn bb_sl_marker(marker: u32) -> i32;
    fn bb_sl_end();
    fn bb_sl_generation(count: u32, width: u32, height: u32) -> i32;
    fn bb_sl_suspend();
    fn bb_sl_tag_generation(images: *const native::Image, frame: *const native::Frame) -> i32;
    fn bb_sl_presented(stats: *mut u32) -> i32;
    #[cfg(test)]
    pub(super) fn bb_sl_markers(marker: u32) -> u64;
}

#[derive(Default)]
pub(in crate::gpu) struct Presentation {
    pub status: LatencyStatus,
    pub request: LatencySettings,
    hudless: Option<wgpu::TextureView>,
    encoded_scene: Option<wgpu::TextureView>,
    raw_ui: Option<wgpu::TextureView>,
    coverage: Option<wgpu::TextureView>,
    ui: Option<wgpu::TextureView>,
    copy: Option<Composite>,
    token_open: bool,
    failed: bool,
}

pub(in crate::gpu) fn capabilities(device: &wgpu::Device) -> LatencyStatus {
    // SAFETY: borrowed physical-device handle belongs to the initialized interposer session.
    let physical =
        unsafe { device.as_hal::<wgpu::hal::api::Vulkan>() }.map(|hal| hal.raw_physical_device().as_raw()).unwrap_or(0);
    let mut caps = [0; 3];
    unsafe { bb_sl_latency_caps(physical as *mut _, caps.as_mut_ptr()) };
    LatencyStatus {
        reflex_available: caps[0] != 0,
        max_generated_frames: caps[1],
        min_dimension: caps[2],
        note: "Ready; frame generation requires VSync off and a temporal upscaler.",
        ..Default::default()
    }
}

impl Presentation {
    pub(in crate::gpu) fn begin(&mut self, request: LatencySettings) -> bool {
        self.request = request;
        // A skipped/minimized frame must not leave a token to reuse on the next CPU simulation.
        if self.token_open {
            unsafe { bb_sl_end() };
        }
        let mode = match request.generated_frames > 0 && request.reflex == blackbox_gfx::ReflexMode::Off {
            true => blackbox_gfx::ReflexMode::On,
            false => request.reflex,
        };
        // SAFETY: this renderer owns SL; all marker and option calls are on the app/present thread.
        let result = unsafe { bb_sl_begin(mode as u32, request.frame_limit_us) };
        self.token_open = true;
        self.status.driver_pacing = self.status.reflex_available && result == 0;
        self.status.driver_pacing
    }

    pub(in crate::gpu) fn render_start(&mut self) {
        if !self.token_open {
            self.begin(self.request);
        }
        unsafe { bb_sl_render_start() };
    }

    pub(in crate::gpu) fn marker(&self, marker: u32) {
        if self.token_open {
            unsafe { bb_sl_marker(marker) };
        }
    }

    pub(in crate::gpu) fn finish(&mut self) {
        let mut stats = [0; 2];
        let result = unsafe {
            let result = bb_sl_presented(stats.as_mut_ptr());
            bb_sl_end();
            result
        };
        self.token_open = false;
        self.status.presented_frames = stats[0];
        self.status.presented_total =
            (result == 0).then(|| self.status.presented_total.unwrap_or(0) + u64::from(stats[0]));
        if self.status.generation_active && (stats[1] != 0 || result != 0) {
            log::error!("DLSS frame generation state query {result}, runtime status: {}", stats[1]);
            self.failed = true;
            self.disable();
            self.status.note = "Frame generation stopped after a runtime error; restart to retry.";
        }
    }

    pub(in crate::gpu) fn disable(&mut self) {
        if self.status.generation_active {
            unsafe { bb_sl_generation(0, 0, 0) };
        }
        self.status.generation_active = false;
    }

    pub(in crate::gpu) fn suspend(&mut self, device: &wgpu::Device) {
        self.disable();
        // SL releases its asynchronous present work before the app can replace a swapchain or tagged images.
        unsafe { bb_sl_suspend() };
        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        self.hudless = None;
        self.encoded_scene = None;
        self.raw_ui = None;
        self.coverage = None;
        self.ui = None;
        self.copy = None;
    }
}

impl Renderer {
    pub(in crate::gpu) fn prepare_generation(&mut self, frame: &blackbox_gfx::FrameParams) {
        if self.dlss.is_none() {
            return;
        }
        let Output::Window { config, .. } = &self.output else { return };
        let request = self.presentation.request;
        let allowed = !self.presentation.failed
            && request.gameplay
            && request.generated_frames > 0
            && request.generated_frames <= self.presentation.status.max_generated_frames
            && matches!(config.present_mode, wgpu::PresentMode::AutoNoVsync | wgpu::PresentMode::Immediate)
            // Streamline 2.14.16 DLSS-G does not support FP16/scRGB swapchains.
            && config.format != wgpu::TextureFormat::Rgba16Float
            && self.temporal_wanted()
            && matches!(frame.projection, Projection::PerspectiveInfiniteReverse { .. })
            && config.width >= self.presentation.status.min_dimension
            && config.height >= self.presentation.status.min_dimension;
        if !allowed {
            self.presentation.disable();
            self.presentation.status.note =
                "Frame generation inactive: needs gameplay, VSync off, SDR/HDR10 output and DLSS/FSR 3.";
            return;
        }
        let result = unsafe { bb_sl_generation(request.generated_frames, config.width, config.height) };
        self.presentation.status.generation_active = result == 0;
        if result == 0 {
            self.presentation.status.note = "Frame generation configured; suspended while unfocused by the SDK.";
        }
        if result != 0 {
            self.presentation.failed = true;
            self.presentation.status.note = "Frame generation configuration failed; restart to retry.";
            log::warn!("slDLSSGSetOptions failed ({result})");
        }
    }

    pub(in crate::gpu) fn generation_target(&mut self, onscreen: bool) -> Option<wgpu::TextureView> {
        let hdr10 = self.output.format() == wgpu::TextureFormat::Rgb10a2Unorm;
        if !onscreen || (!self.presentation.status.generation_active && !hdr10) {
            return None;
        }
        let size = self.surface_size();
        let format = if hdr10 { wgpu::TextureFormat::Rgba16Float } else { self.output.format() };
        let matches = self.presentation.hudless.as_ref().is_some_and(|v| {
            v.texture().width() == size.0 && v.texture().height() == size.1 && v.texture().format() == format
        });
        if !matches {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("DLSS-G post-processed HUD-less color"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            self.presentation.hudless = Some(texture.create_view(&Default::default()));
            let desc = wgpu::TextureDescriptor {
                label: Some("DLSS-G separate UI color and alpha"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            };
            self.presentation.raw_ui = Some(self.device.create_texture(&desc).create_view(&Default::default()));
            self.presentation.ui = Some(self.device.create_texture(&desc).create_view(&Default::default()));
            self.presentation.coverage = Some(
                self.device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("DLSS-G UI coverage including additive elements"),
                        // SDK 2.14.16 Vulkan getFormat() maps R16_SFLOAT but not R8_UNORM.
                        format: wgpu::TextureFormat::R16Float,
                        ..desc
                    })
                    .create_view(&Default::default()),
            );
            self.presentation.encoded_scene = hdr10.then(|| {
                let desc = wgpu::TextureDescriptor {
                    label: Some("DLSS-G HDR10 HUDless PQ color"),
                    format: wgpu::TextureFormat::Rgb10a2Unorm,
                    ..desc
                };
                self.device.create_texture(&desc).create_view(&Default::default())
            });
            self.presentation.copy = None;
        }
        self.presentation.hudless.clone()
    }

    pub(in crate::gpu) fn composite_generation(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        hudless: &wgpu::TextureView,
        output: &wgpu::TextureView,
    ) {
        let raw_ui = self.presentation.raw_ui.as_ref().expect("UI target accompanies HUDless").clone();
        let ui = self.presentation.ui.as_ref().expect("UI composition target").clone();
        let coverage = self.presentation.coverage.as_ref().expect("UI coverage target").clone();
        for view in [&raw_ui, &coverage] {
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear separate UI"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        self.ui.encode_with_coverage(&self.device, &self.queue, encoder, &raw_ui, &coverage, self.surface_size());
        let copy = self.presentation.copy.get_or_insert_with(|| Composite::new(&self.device, self.output.format()));
        let encoded_scene = self.presentation.encoded_scene.clone();
        let mut outputs = vec![output, &ui];
        outputs.extend(encoded_scene.as_ref());
        copy.encode(&self.device, encoder, hudless, &raw_ui, &outputs);
        if !self.presentation.status.generation_active {
            return;
        }
        let Some(scene) = self.targets.offscreen() else {
            self.presentation.disable();
            return;
        };
        let Some(frame) = self.temporal.current() else {
            self.presentation.disable();
            return;
        };
        let Some(motion) = &scene.motion else {
            self.presentation.disable();
            return;
        };
        let views = [&scene.depth_view, &motion.view, encoded_scene.as_ref().unwrap_or(hudless), &coverage];
        encoder.transition_resources(
            std::iter::empty(),
            views.into_iter().map(|view| wgpu::TextureTransition {
                texture: view.texture(),
                selector: None,
                state: wgpu::TextureUses::RESOURCE,
            }),
        );
        let Some(images) = views.map(|v| native::image(v, false)).into_iter().collect::<Option<Vec<_>>>() else {
            self.presentation.failed = true;
            self.presentation.disable();
            self.presentation.status.note = "Frame generation stopped: a tagged resource format is unsupported.";
            return;
        };
        let constants = native::frame_data(frame, 0, 0);
        // The frame's final submission executes before SL's present hook. Tagged resources remain
        // immutable through present on this same queue, including the original motion/depth buffers.
        let result = unsafe { bb_sl_tag_generation(images.as_ptr(), &constants) };
        if result != 0 {
            log::error!("DLSS-G resource tagging failed ({result})");
            self.presentation.failed = true;
            self.presentation.disable();
            self.presentation.status.note = "Frame generation stopped after resource tagging failed; restart to retry.";
        }
    }
}
