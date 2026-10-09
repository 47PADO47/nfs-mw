//! The user's graphics options: post effects, anti-aliasing, render scale, upscaling and ray tracing.
//!
//! All pure data and pure helpers. What a renderer can actually run is decided by
//! [`resolve`](crate::resolve) against its [`Capabilities`](crate::Capabilities).

pub mod post;
pub mod ray_tracing;
pub mod render_scale;
pub mod upscale;

pub use post::{
    Antialiasing, DEFAULT_BLOOM_THRESHOLD, MAX_BLOOM_INTENSITY, MAX_BLOOM_THRESHOLD, MAX_EXPOSURE, MIN_EXPOSURE,
    PostEffect, PostSettings, Tonemap,
};
pub use ray_tracing::RayTracing;
pub use render_scale::{DEFAULT_RENDER_SCALE, MAX_RENDER_SCALE, MIN_RENDER_SCALE, clamp_render_scale, scaled_size};
pub use upscale::{
    DEFAULT_UPSCALE_SHARPNESS, MAX_TEXTURE_LOD_BIAS, MIN_TEXTURE_LOD_BIAS, UpscaleQuality, Upscaler,
    clamp_texture_lod_bias, clamp_upscale_sharpness, fsr1_active, rcas_stops, suggested_temporal_texture_lod_bias,
    suggested_texture_lod_bias,
};

/// Everything a user can ask of the renderer, in one value. This is what is *requested*; the
/// renderer's capabilities decide what is *effective* (see [`resolve`](crate::resolve)).
///
/// [`Default`] asks for nothing: no post effects, bilinear upscaling at native size, no ray tracing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphicsSettings {
    /// Post effects, including the anti-aliasing method.
    pub post: PostSettings,
    pub upscaler: Upscaler,
    /// Sets the render size of a temporal upscaler.
    pub upscale_quality: UpscaleQuality,
    /// Render size relative to the output, for the spatial upscalers ([`MIN_RENDER_SCALE`]..=[`MAX_RENDER_SCALE`]).
    pub render_scale: f32,
    /// RCAS sharpness of FSR 1, 0.0..=1.0.
    pub upscale_sharpness: f32,
    pub ray_tracing: RayTracing,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            post: PostSettings::default(),
            upscaler: Upscaler::default(),
            upscale_quality: UpscaleQuality::default(),
            render_scale: DEFAULT_RENDER_SCALE,
            upscale_sharpness: DEFAULT_UPSCALE_SHARPNESS,
            ray_tracing: RayTracing::Off,
        }
    }
}

impl GraphicsSettings {
    /// The values clamped into their ranges; non-finite values fall back to the defaults.
    pub fn sanitized(self) -> Self {
        Self {
            post: self.post.sanitized(),
            render_scale: clamp_render_scale(self.render_scale),
            upscale_sharpness: clamp_upscale_sharpness(self.upscale_sharpness),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_asks_for_nothing() {
        let s = GraphicsSettings::default();
        assert!(s.post.effects().is_empty());
        assert_eq!((s.upscaler, s.render_scale, s.ray_tracing), (Upscaler::Bilinear, 1.0, RayTracing::Off));
        assert_eq!(s.sanitized(), s);
    }

    #[test]
    fn sanitizing_clamps_every_numeric_field() {
        let wild = GraphicsSettings {
            post: PostSettings { exposure: 100.0, ..PostSettings::default() },
            render_scale: 9.0,
            upscale_sharpness: f32::NAN,
            ..GraphicsSettings::default()
        };
        let clean = wild.sanitized();
        assert_eq!(clean.post.exposure, MAX_EXPOSURE);
        assert_eq!(clean.render_scale, MAX_RENDER_SCALE);
        assert_eq!(clean.upscale_sharpness, DEFAULT_UPSCALE_SHARPNESS);
    }
}
