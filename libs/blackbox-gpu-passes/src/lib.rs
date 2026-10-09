//! Reusable wgpu passes for EA Black Box renderers.
//!
//! Every pass takes plain wgpu inputs (`&Device`, `&Queue`, a `&mut CommandEncoder` or a `RenderPass`,
//! texture views, target formats) and [`blackbox_gfx`] data (`EffectLayer`, `UiLayer`, ...), and knows
//! nothing about the renderer that calls it. The native renderer calls them from its frame; a renderer
//! built on another engine can call the same code from its own render loop with its own encoder.
//!
//! See the README for the call sequence of each pass.

mod world;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use world::{DEPTH_FORMAT, Globals, HDR_FORMAT, WorldBindings, create_depth, write_mask};
