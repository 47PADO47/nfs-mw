//! AMD FidelityFX Super Resolution 3.1 temporal upscaler for wgpu.
//!
//! A port to WGSL of the upscaler of the MIT-licensed FidelityFX SDK (v1.1.4, FSR 3.1.4), not of its
//! frame generation. It runs on every wgpu backend that supports compute shaders (Vulkan, Direct3D 12,
//! Metal), in 32-bit floats and without subgroup operations. See the README for the pinned SDK files and
//! the differences from AMD's version.
//!
//! The renderer renders at a lower resolution with a different sub-pixel jitter each frame
//! ([`jitter`]), samples world textures with the [`QualityMode::mip_bias`], and each frame calls
//! [`Fsr3Context::dispatch`] with the colour, depth and motion vectors; the upscaler accumulates the
//! jittered frames into an image at the output resolution.

mod config;
mod constants;
mod context;
mod debug;
mod error;
mod inputs;
pub mod jitter;
mod passes;
mod pipelines;
mod quality;
mod resources;
pub mod shaders;

pub use config::{DepthConvention, Fsr3Config, MotionVectorLayout, Tuning};
pub use constants::{CONSTANTS_SIZE, Constants, device_to_view_depth, view_depth};
pub use context::Fsr3Context;
pub use debug::DebugTexture;
pub use error::Fsr3Error;
pub use inputs::{Fsr3Inputs, Fsr3Outputs, MOTION_VECTOR_SCALE_PIXELS, motion_vector_scale_ndc};
pub use quality::{QualityMode, mip_bias, render_size_for_ratio};

#[cfg(test)]
mod shader_tests;
