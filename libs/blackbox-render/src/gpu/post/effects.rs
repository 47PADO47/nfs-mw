//! Building the post-effect passes from [`PostSettings`] and the renderer's switch for them.

use super::bloom::BloomPass;
use super::fxaa::FxaaPass;
use super::tonemap::TonemapPass;
use super::{PostChain, PostPass};
use crate::gpu::Renderer;
use crate::{PostEffect, PostSettings};

/// The passes for `settings` in execution order.
fn build(
    device: &wgpu::Device,
    settings: &PostSettings,
    profile: Option<&super::treatment::Profile>,
    cache: Option<&wgpu::PipelineCache>,
) -> Vec<Box<dyn PostPass>> {
    let mut passes: Vec<Box<dyn PostPass>> = Vec::new();
    for effect in settings.effects() {
        match effect {
            PostEffect::SceneTreatment => {
                if settings.colour_treatment <= 0.0 || profile.is_none() {
                    continue;
                }
                passes.push(Box::new(super::treatment::TreatmentPass::new(
                    device,
                    settings,
                    profile.expect("profile checked above"),
                    cache,
                )))
            }
            PostEffect::Bloom => passes.push(Box::new(BloomPass::new(device, settings, cache))),
            PostEffect::Fxaa => passes.push(Box::new(FxaaPass::new(device, cache))),
            PostEffect::Tonemap => passes.push(Box::new(TonemapPass::new(device, settings, cache))),
        }
    }
    passes
}

impl PostChain {
    /// Profile changes invalidate the effect list even when the user settings stay the same.
    pub(super) fn set_treatment(&mut self, device: &wgpu::Device, profile: Option<super::treatment::Profile>) {
        self.profile = profile;
        let fresh = build(device, &self.settings, self.profile.as_ref(), self.cache.as_ref());
        let count = fresh.len();
        self.passes.splice(..self.effect_count, fresh);
        self.effect_count = count;
    }

    /// Replace the effect passes at the front of the chain with those `settings` ask for. Returns
    /// whether anything changed. Other passes stay.
    pub(in crate::gpu) fn set_effects(&mut self, device: &wgpu::Device, settings: PostSettings) -> bool {
        let settings = settings.sanitized();
        if settings == self.settings {
            return false;
        }
        let fresh = build(device, &settings, self.profile.as_ref(), self.cache.as_ref());
        let count = fresh.len();
        self.passes.splice(..self.effect_count, fresh);
        self.effect_count = count;
        self.settings = settings;
        true
    }

    pub(in crate::gpu) fn effect_settings(&self) -> PostSettings {
        self.settings
    }
}

impl Renderer {
    pub(crate) fn set_scene_treatment(
        &mut self,
        profile: Option<&blackbox_gfx::ColourTreatment>,
        gain: Option<&blackbox_gfx::RgbaImage>,
    ) {
        let profile = profile.filter(|p| p.is_valid()).map(|p| super::treatment::Profile {
            settings: *p,
            gain: super::treatment::gain_texture(&self.device, &self.queue, gain),
            luma_format: self.hdr_format,
        });
        self.post.set_treatment(&self.device, profile);
        self.refresh_targets();
    }

    /// Turn the post-process effects (bloom, tone mapping, FXAA) on or off and set their strength.
    /// They run on the scene at the internal render size, before the output; the UI is never
    /// affected. The default [`PostSettings`] runs none, so the frame is the plain clamped scene.
    /// Values are clamped into their ranges; setting the current value again does nothing.
    pub fn set_post_effects(&mut self, settings: PostSettings) {
        if self.post.set_effects(&self.device, settings) {
            self.refresh_targets();
        }
    }

    /// The post-process settings in effect (after clamping).
    pub fn post_effects(&self) -> PostSettings {
        self.post.effect_settings()
    }
}
