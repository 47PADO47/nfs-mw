//! The post-process chain: an ordered list of fullscreen passes between the HDR scene image and
//! the output image. The UI is drawn after the chain, so it is never post-processed or upscaled.
//!
//! The last pass writes the output image itself. The post effects (bloom, tone mapping, FXAA; see
//! [`effects`]) sit at the front, and later passes (upscaling) are inserted with [`PostChain::insert`].
//! Passes that write the output size (the spatial upscalers) are always kept after the ones that write
//! the render size. Only when the last pass cannot do that exactly does the built-in [`resolve`] copy
//! follow it (see [`plan::needs_resolve`]), and with no pass at all the copy is the whole chain.
//!
//! A temporal upscaler (FSR 3) is the exception: it runs first, on the scene as drawn, so bloom and tone
//! mapping see the upscaled image and everything after it writes the output size. It is kept apart from the
//! other passes ([`PostChain::set_temporal`]) and skipped for a frame that has no temporal data.

mod bloom;
pub(in crate::gpu) mod display;
#[cfg(test)]
mod display_tests;
#[cfg(test)]
mod display_vectors;
mod distortion;
#[cfg(test)]
mod distortion_tests;
#[cfg(test)]
mod effect_tests;
mod effects;
mod filter;
mod focus;
mod fsr1;
#[cfg(test)]
mod fsr1_tests;
mod fsr3;
mod fusion;
mod fxaa;
mod grade;
mod plan;
mod radial_blur;
#[cfg(test)]
mod radial_blur_tests;
pub(in crate::gpu) mod resolve;
#[cfg(test)]
mod tests;
mod tonemap;
mod treatment;
mod treatment_composite;

use super::sized::SizedPair;
use super::targets::FrameTargets;
use super::temporal::TemporalFrame;
use plan::{Source, Target};

pub(super) use fsr1::{EASU as FSR1_EASU, RCAS as FSR1_RCAS, RcasScale, passes as fsr1_passes};
pub(super) use fsr3::Fsr3Pass;

/// How big the image a pass writes is, when it does not write the final output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Extent {
    /// The internal render size (the scene image's size).
    Render,
    /// The output size (an upscaling pass writes this).
    Output,
}

/// Everything a pass may read besides its own input.
pub(super) struct PassContext<'a> {
    pub device: &'a wgpu::Device,
    #[allow(dead_code, reason = "for passes that upload parameters")]
    pub queue: &'a wgpu::Queue,
    /// The scene's HDR colour and its depth buffer (reverse-Z, bindable as a sampled depth
    /// texture), both at the render size. The same for every pass of the frame, whatever its input.
    pub scene: &'a FrameTargets,
    pub output_size: (u32, u32),
    /// The jitter and matrices of the frame when a temporal upscaler works on it, else `None`.
    pub temporal: Option<&'a TemporalFrame>,
}

/// The image a pass reads and the one it writes.
pub(super) struct PassIo<'a> {
    pub input: &'a wgpu::TextureView,
    pub input_size: (u32, u32),
    pub output: &'a wgpu::TextureView,
    pub output_format: wgpu::TextureFormat,
    pub output_size: (u32, u32),
}

pub(super) trait PostPass {
    /// The lens regions to draw, or `None` for no view (nothing to draw).
    fn set_distortion(&mut self, _view: Option<&wgpu::TextureView>, _regions: &[blackbox_gfx::DistortionRegion]) {}
    fn name(&self) -> &'static str;

    /// Whether this pass has anything to draw. An inactive pass stays in the chain (so it is not rebuilt when it
    /// becomes active again) but is left out of every frame's plan and of [`PostChain::names`].
    fn active(&self) -> bool {
        true
    }
    fn set_focus(&mut self, _settings: blackbox_gfx::FocusFilter) {}
    fn set_display(&mut self, _mapping: Option<display::Mapping>) {}

    /// The exposure, when this is the tone-mapping pass: a pass before it may run it as part of its last draw
    /// (see [`fusion`]).
    fn tonemap_exposure(&self) -> Option<f32> {
        None
    }

    /// Run a tone mapping with `exposure` as part of this pass's last draw, or none for `None` (the chain tells
    /// every pass on every frame). Returns whether this pass can do so; the chain drops the tone-mapping pass
    /// after a pass that returns true.
    fn absorb_tonemap(&mut self, _exposure: Option<f32>) -> bool {
        false
    }

    /// The size of the image this pass writes when it is not the last one.
    fn extent(&self) -> Extent {
        Extent::Render
    }

    fn encode(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder);
    fn encode_diagnostic(&mut self, ctx: &PassContext<'_>, io: &PassIo<'_>, encoder: &mut wgpu::CommandEncoder) {
        self.encode(ctx, io, encoder);
    }
}

/// A ping-pong image between two passes.
struct Scratch {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
}

/// What a ping-pong image was made for: its size and format.
type ScratchKey = ((u32, u32), wgpu::TextureFormat);

impl Scratch {
    fn new(device: &wgpu::Device, size: (u32, u32), format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("post-process scratch"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { _texture: texture, view, format }
    }
}

pub(super) struct PostChain {
    /// The post effects and the other passes, in order. The output copy is not one of them. The final
    /// display pass (see `display.rs`), when there is one, is the last of them.
    passes: Vec<Box<dyn PostPass>>,
    /// The copy to the output, run after `passes` only when the last pass cannot write the output itself.
    resolve: resolve::Resolve,
    /// The temporal upscaler, which runs before `passes` when the frame has temporal data.
    temporal: Option<Box<dyn PostPass>>,
    /// The ping-pong images, kept for two sizes so a frame at each view size reuses its own.
    scratch: [SizedPair<ScratchKey, Scratch>; 2],
    /// How many encodes in a row each ping-pong slot went unused (see [`plan::slot_expired`]).
    scratch_idle: [u32; 2],
    /// [`plan::signature`] of `passes` as of the last encode: the images are freed when it changes.
    signature: u64,
    /// Buffers `encode` refills every frame instead of allocating.
    extents: Vec<Extent>,
    steps: Vec<plan::Step>,
    /// How many passes at the front of `passes` are the post effects of `settings`.
    effect_count: usize,
    settings: crate::PostSettings,
    /// Fuse the tone mapping into the pass before it (see [`fusion`]). Always on outside the tests, which turn
    /// it off to compare against the separate passes.
    fuse: bool,
    profile: Option<treatment::Profile>,
    /// Used by the effect passes built when the settings or the profile change.
    cache: Option<wgpu::PipelineCache>,
    /// The display mapping (HDR output or SDR preview) the chain was given, if any. See `display.rs`.
    pub(in crate::gpu) display: Option<display::Mapping>,
}

impl PostChain {
    pub(super) fn new(device: &wgpu::Device, cache: Option<&wgpu::PipelineCache>) -> Self {
        Self {
            passes: Vec::new(),
            resolve: resolve::Resolve::new(device, cache),
            display: None,
            temporal: None,
            scratch: [SizedPair::new(), SizedPair::new()],
            scratch_idle: [0; 2],
            signature: 0,
            extents: Vec::new(),
            steps: Vec::new(),
            effect_count: 0,
            settings: crate::PostSettings::default(),
            fuse: true,
            profile: None,
            cache: cache.cloned(),
        }
    }

    /// Insert `pass` so it runs at `index` among the passes (0 = first). Past-the-end indices put it at the
    /// end, or right before the display pass when there is one, which always stays last. The post effects
    /// stay in front of every inserted pass, and a pass that writes the output size ignores the index and
    /// goes last (before the display pass), so it follows every render-size pass.
    pub(super) fn insert(&mut self, index: usize, mut pass: Box<dyn PostPass>) {
        pass.set_display(self.display);
        let extents: Vec<Extent> = self.passes.iter().map(|p| p.extent()).collect();
        let index = plan::insert_index(&extents, index.max(self.effect_count), pass.extent());
        let last_scene = self.passes.iter().position(|p| p.name() == "hdr-display").unwrap_or(self.passes.len());
        let mut index = index.min(last_scene);
        if pass.name() != distortion::NAME && pass.name() != "hdr-display" {
            let lens = self.passes.iter().position(|p| p.name() == distortion::NAME).unwrap_or(self.passes.len());
            index = index.min(lens);
        }
        self.passes.insert(index, pass);
    }

    /// Whether a pass named `name` is in the chain.
    pub(super) fn has(&self, name: &str) -> bool {
        self.passes.iter().any(|p| p.name() == name)
    }

    /// Remove every pass named `name`.
    pub(super) fn remove(&mut self, name: &str) {
        self.passes.retain(|p| p.name() != name);
    }

    /// Whether any pass runs (post effects, upscalers; an inactive pass does not count). When none does, the
    /// scene can be drawn straight into the output.
    pub(super) fn has_passes(&self) -> bool {
        self.temporal.is_some() || self.passes.iter().any(|p| p.active())
    }

    /// Put the temporal upscaler first in the chain, or take it out with `None`.
    pub(super) fn set_temporal(&mut self, pass: Option<Box<dyn PostPass>>) {
        self.temporal = pass;
    }

    /// Whether a temporal upscaler is in the chain.
    pub(super) fn has_temporal(&self) -> bool {
        self.temporal.is_some()
    }

    /// Free the ping-pong images (they are recreated when a pass needs them).
    pub(super) fn release_scratch(&mut self) {
        for images in &mut self.scratch {
            images.clear();
        }
        self.scratch_idle = [0; 2];
    }

    /// How many ping-pong images are allocated.
    #[cfg(test)]
    pub(super) fn scratch_count(&self) -> usize {
        self.scratch.iter().map(|images| images.len()).sum()
    }

    /// The names of the passes that run, in execution order, not counting the output copy (see
    /// [`PostChain::encode`]).
    pub(super) fn names(&self) -> Vec<&'static str> {
        let active = self.passes.iter().filter(|p| p.active());
        self.temporal.iter().chain(active).map(|p| p.name()).collect()
    }

    /// Surface diagnostics bypass grading/effects without changing saved or requested settings.
    pub(super) fn encode_diagnostic(
        &mut self,
        ctx: &PassContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        output: (&wgpu::TextureView, wgpu::TextureFormat),
    ) {
        let io = PassIo {
            input: &ctx.scene.color_view,
            input_size: ctx.scene.size,
            output: output.0,
            output_format: output.1,
            output_size: ctx.output_size,
        };
        if let Some(pass) = self.passes.iter_mut().find(|p| p.name() == "hdr-display") {
            pass.encode_diagnostic(ctx, &io, encoder);
            return;
        }
        self.resolve.encode(ctx, &io, encoder);
    }

    /// Record every pass. The last one writes `output`: the output copy follows the passes only when
    /// [`plan::needs_resolve`] says the last pass cannot write it exactly.
    pub(super) fn encode(
        &mut self,
        ctx: &PassContext<'_>,
        encoder: &mut wgpu::CommandEncoder,
        output: (&wgpu::TextureView, wgpu::TextureFormat),
    ) {
        let Self { passes, resolve, temporal, scratch, scratch_idle, signature, extents, steps, fuse, .. } = self;
        let scene = ctx.scene;
        // A different set of passes (settings changed) frees the images; the same set with fewer passes running
        // on some frames (the mirror capture has no temporal upscaler) keeps them, see `slot_expired` below.
        let now = plan::signature(passes.iter().map(|pass| pass.name()));
        if now != *signature {
            *signature = now;
            *scratch_idle = [0; 2];
            for images in scratch.iter_mut() {
                images.clear();
            }
        }
        // The temporal upscaler only runs on a frame that carries its inputs.
        let upscaler = temporal.as_mut().filter(|_| ctx.temporal.is_some() && scene.motion.is_some());
        let temporal_last = upscaler.is_some() && !passes.iter().any(|pass| pass.active());
        let mut chain: Vec<&mut dyn PostPass> = Vec::with_capacity(passes.len() + 2);
        if let Some(pass) = upscaler {
            chain.push(pass.as_mut());
        }
        for pass in passes.iter_mut().filter(|pass| pass.active()) {
            chain.push(pass.as_mut());
        }
        fusion::fuse_tonemap(&mut chain, *fuse);
        extents.clear();
        extents.extend(chain.iter().map(|pass| pass.extent()));
        if plan::needs_resolve(extents.as_slice(), temporal_last, scene.size, ctx.output_size, output.1) {
            chain.push(resolve as &mut dyn PostPass);
        }
        plan::plan_into(chain.len(), steps);
        // A slot the chain no longer uses keeps its images for a while: another view of the same frame (the mirror)
        // may use it again at once. It is freed only after many encodes without a use.
        for (slot, images) in scratch.iter_mut().enumerate() {
            let used = steps.iter().any(|step| step.target == Target::Scratch(slot));
            if plan::slot_expired(&mut scratch_idle[slot], used) {
                images.clear();
            }
        }
        // The key of the image each slot was last written with in this frame.
        let mut written: [Option<ScratchKey>; 2] = [None, None];
        // Once a pass has upscaled, every pass after it writes the output size.
        let mut upscaled = false;
        for (pass, step) in chain.into_iter().zip(steps.iter().copied()) {
            if let Target::Scratch(slot) = step.target {
                let size = match plan::written_extent(upscaled, pass.extent()) {
                    Extent::Render => scene.size,
                    Extent::Output => ctx.output_size,
                };
                let key = (size, scene.color_format);
                scratch[slot].get_or_make(key, || {
                    let _trace = blackbox_trace::scope("Renderer CPU: post scratch image created");
                    Scratch::new(ctx.device, size, scene.color_format)
                });
                written[slot] = Some(key);
            }
            let (input, input_size) = match step.source {
                Source::Scene => (&scene.color_view, scene.size),
                Source::Scratch(slot) => {
                    let key = written[slot].expect("written earlier");
                    scratch[slot].get(key).map(|s| (&s.view, key.0)).expect("written earlier")
                }
            };
            let (view, format, size) = match step.target {
                Target::Output => (output.0, output.1, ctx.output_size),
                Target::Scratch(slot) => {
                    let key = written[slot].expect("created above");
                    scratch[slot].get(key).map(|s| (&s.view, s.format, key.0)).expect("created above")
                }
            };
            let io = PassIo { input, input_size, output: view, output_format: format, output_size: size };
            pass.encode(ctx, &io, encoder);
            upscaled |= pass.extent() == Extent::Output;
        }
    }
}
