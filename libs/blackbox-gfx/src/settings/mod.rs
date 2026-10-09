//! The user's graphics options: post effects, anti-aliasing, render scale and upscaling.
//!
//! All pure data and pure helpers. What a renderer can actually run is decided by
//! [`resolve`](crate::resolve) against its [`Capabilities`](crate::Capabilities).

pub mod post;
pub mod render_scale;
pub mod upscale;

pub use post::{
    Antialiasing, DEFAULT_BLOOM_THRESHOLD, MAX_BLOOM_INTENSITY, MAX_BLOOM_THRESHOLD, MAX_EXPOSURE, MIN_EXPOSURE,
    PostEffect, PostSettings, Tonemap,
};
pub use render_scale::{DEFAULT_RENDER_SCALE, MAX_RENDER_SCALE, MIN_RENDER_SCALE, clamp_render_scale, scaled_size};
pub use upscale::{
    DEFAULT_UPSCALE_SHARPNESS, MAX_TEXTURE_LOD_BIAS, MIN_TEXTURE_LOD_BIAS, Upscaler, clamp_texture_lod_bias,
    clamp_upscale_sharpness, fsr1_active, rcas_stops, suggested_texture_lod_bias,
};
