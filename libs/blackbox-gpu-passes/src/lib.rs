//! Reusable wgpu passes for EA Black Box renderers.
//!
//! Every pass takes plain wgpu inputs (`&Device`, `&Queue`, a `&mut CommandEncoder` or a `RenderPass`,
//! texture views, target formats) and [`blackbox_gfx`] data (`EffectLayer`, `UiLayer`, ...), and knows
//! nothing about the renderer that calls it. The native renderer calls them from its frame; a renderer
//! built on another engine can call the same code from its own render loop with its own encoder.
//!
//! See the README for the call sequence of each pass.

mod batch;
mod effects;
#[cfg(test)]
mod effects_tests;
mod filter;
#[cfg(test)]
mod filter_tests;
mod fsr1;
#[cfg(test)]
mod fsr1_tests;
#[cfg(test)]
mod shader_tests;
#[cfg(test)]
mod soft_particle_tests;
mod soft_particles;
mod textured_effects;
mod ui;
#[cfg(test)]
mod ui_tests;
mod world;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use batch::Batch;

/// The WGSL of the world effects (`vs_main` and the `fs_*` stages), for renderers that build their own
/// effect pipelines. [`TexturedEffects`] and the renderer's effect pipelines use the same source.
pub const EFFECTS_WGSL: &str = include_str!("shaders/effects.wgsl");

pub use effects::{Effects, SoftDraw};
pub use filter::{Blend, Draw, Filter, POST_COMMON_WGSL, Params, filter_source};
pub use fsr1::{FSR1_EASU, FSR1_RCAS, Fsr1Io, Fsr1Pass, Fsr1Stage, RcasScale, fsr1_passes};
pub use soft_particles::SoftParticles;
pub use textured_effects::TexturedEffects;
pub use ui::{UiPass, UiTextureError};
pub use world::{DEPTH_FORMAT, EFFECT_VERTEX_ATTRIBUTES, Globals, HDR_FORMAT, WorldBindings, create_depth, write_mask};
