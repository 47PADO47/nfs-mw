//! Building the post-effect passes from [`PostSettings`] and the renderer's switch for them.

use super::bloom::BloomPass;
use super::fxaa::FxaaPass;
use super::tonemap::TonemapPass;
use super::{PostChain, PostPass};
use crate::gpu::Renderer;
use crate::{PostEffect, PostSettings};

/// The passes for `settings` in execution order.
fn build(device: &wgpu::Device, settings: &PostSettings) -> Vec<Box<dyn PostPass>> {
    let mut passes: Vec<Box<dyn PostPass>> = Vec::new();
    for effect in settings.effects() {
        match effect {
            PostEffect::Bloom => passes.push(Box::new(BloomPass::new(device, settings))),
            PostEffect::Fxaa => passes.push(Box::new(FxaaPass::new(device))),
            PostEffect::Tonemap => passes.push(Box::new(TonemapPass::new(device, settings))),
        }
    }
    passes
}

impl PostChain {
    /// Replace the effect passes at the front of the chain with those `settings` ask for. Returns
    /// whether anything changed. Other passes and the resolve pass stay.
    pub(in crate::gpu) fn set_effects(&mut self, device: &wgpu::Device, settings: PostSettings) -> bool {
        let settings = settings.sanitized();
        if settings == self.settings {
            return false;
        }
        let fresh = build(device, &settings);
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
    /// Turn the post-process effects (bloom, tone mapping, FXAA) on or off and set their strength.
    /// They run on the scene at the internal render size, before the final resolve; the UI is never
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
