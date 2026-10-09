//! AMD FidelityFX Super Resolution 3.1 temporal upscaler for wgpu.
//!
//! A port to WGSL of the upscaler of the MIT-licensed FidelityFX SDK (v1.1.4, FSR 3.1.4), not of its
//! frame generation. See the README for the pinned SDK files and the differences from AMD's version.

mod config;
mod constants;
mod error;
mod inputs;
pub mod jitter;
mod quality;

pub use config::{DepthConvention, Fsr3Config, MotionVectorLayout, Tuning};
pub use constants::{CONSTANTS_SIZE, Constants, device_to_view_depth, view_depth};
pub use error::Fsr3Error;
pub use inputs::{Fsr3Inputs, Fsr3Outputs, MOTION_VECTOR_SCALE_PIXELS, motion_vector_scale_ndc};
pub use quality::{QualityMode, mip_bias, render_size_for_ratio};
