//! Choosing the upscaler and keeping the post chain and the texture sampling in step with it.

use super::Renderer;
use super::post::{FSR1_EASU, FSR1_RCAS, RcasScale, fsr1_passes};
use crate::upscale::{fsr1_active, rcas_stops};
use crate::{DEFAULT_UPSCALE_SHARPNESS, Upscaler, clamp_texture_lod_bias, clamp_upscale_sharpness};

/// The upscaling state of a renderer.
pub(super) struct Upscale {
    upscaler: Upscaler,
    sharpness: f32,
    /// Added to the mip level of every world texture sample (the scene shader reads it from the globals).
    pub texture_lod_bias: f32,
    rcas: RcasScale,
}

impl Default for Upscale {
    fn default() -> Self {
        Self {
            upscaler: Upscaler::default(),
            sharpness: DEFAULT_UPSCALE_SHARPNESS,
            texture_lod_bias: 0.0,
            rcas: RcasScale::new(),
        }
    }
}

impl Renderer {
    /// Choose how a scene drawn below the surface size (a render scale under 1.0) is brought back up.
    /// [`Upscaler::Bilinear`] is the default. At a render scale of 1.0 or more no upscaler runs.
    pub fn set_upscaler(&mut self, upscaler: Upscaler) {
        if upscaler == self.upscale.upscaler {
            return;
        }
        self.upscale.upscaler = upscaler;
        self.sync_upscale_passes();
    }

    /// The chosen upscaler.
    pub fn upscaler(&self) -> Upscaler {
        self.upscale.upscaler
    }

    /// Set the FSR 1 sharpening strength, 0.0 (off) to 1.0 (strongest); clamped. It has no effect on
    /// the other upscalers.
    pub fn set_upscale_sharpness(&mut self, sharpness: f32) {
        let sharpness = clamp_upscale_sharpness(sharpness);
        if sharpness == self.upscale.sharpness {
            return;
        }
        self.upscale.sharpness = sharpness;
        self.sync_upscale_passes();
    }

    /// The FSR 1 sharpening strength (after clamping).
    pub fn upscale_sharpness(&self) -> f32 {
        self.upscale.sharpness
    }

    /// Add `bias` to the mip level of the world's texture samples (clamped to
    /// [`MIN_TEXTURE_LOD_BIAS`](crate::MIN_TEXTURE_LOD_BIAS)..=[`MAX_TEXTURE_LOD_BIAS`](crate::MAX_TEXTURE_LOD_BIAS)).
    /// A negative bias sharpens textures; use [`suggested_texture_lod_bias`](crate::suggested_texture_lod_bias)
    /// for the current render scale while upscaling. Effects and the UI are not biased.
    pub fn set_texture_lod_bias(&mut self, bias: f32) {
        self.upscale.texture_lod_bias = clamp_texture_lod_bias(bias);
    }

    /// The texture LOD bias in use (after clamping).
    pub fn texture_lod_bias(&self) -> f32 {
        self.upscale.texture_lod_bias
    }

    /// Whether the FSR 1 passes are in the chain now (the upscaler is FSR 1 and the render scale is below 1.0).
    pub fn fsr1_active(&self) -> bool {
        fsr1_active(self.upscale.upscaler, self.render_scale)
    }

    /// Put the post chain in line with the upscaler, the render scale and the sharpness, and the scene
    /// targets with the chain.
    pub(super) fn sync_upscale_passes(&mut self) {
        self.sync_fsr1_passes();
        self.refresh_targets();
    }

    fn sync_fsr1_passes(&mut self) {
        let active = self.fsr1_active();
        let stops = rcas_stops(self.upscale.sharpness);
        if let Some(stops) = stops {
            self.upscale.rcas.set_stops(stops);
        }
        let sharpen = active && stops.is_some();
        if (self.post.has(FSR1_EASU), self.post.has(FSR1_RCAS)) == (active, sharpen) {
            return;
        }
        self.post.remove(FSR1_EASU);
        self.post.remove(FSR1_RCAS);
        if !active {
            return;
        }
        for pass in fsr1_passes(&self.device, &self.upscale.rcas, sharpen) {
            self.post.insert(usize::MAX, pass);
        }
    }
}
