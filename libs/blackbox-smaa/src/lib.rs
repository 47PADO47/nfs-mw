//! SMAA 1x post-process antialiasing for wgpu, ported to WGSL.
//!
//! Subpixel Morphological Antialiasing (Jimenez et al., <http://www.iryoku.com/smaa>) at the HIGH preset, the
//! quality the `smaa` crate 0.21.0 ships. The three passes (luma edge detection, blending weights, neighbourhood
//! blending) are WGSL under `src/shaders/`, one module per pass, so every pipeline takes a [`wgpu::PipelineCache`].
//!
//! Per frame: [`SmaaTarget::start_frame`] gives a [`SmaaFrame`] that derefs to the view to render the scene into,
//! and [`SmaaFrame::resolve`] (or dropping the frame) runs the three passes into the output view.
//!
//! The area and search tables are the SMAA reference's precomputed textures, uploaded once per target.
//! Licence: the SMAA reference licence and the smaa crate's MIT licence, both in `LICENSE-SMAA`.

#![deny(missing_docs)]

mod area;
mod area_diag;
mod bindings;
mod frame;
mod pipelines;
mod resources;
mod search;
mod shaders;
mod target;
mod targets;

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod shader_tests;
#[cfg(test)]
mod tests;

pub use frame::SmaaFrame;
pub use target::SmaaTarget;

/// Antialiasing mode. Higher values look better and cost more.
#[non_exhaustive]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SmaaMode {
    /// No antialiasing: the frame renders straight into the output view.
    Disabled,
    /// SMAA 1x.
    Smaa1X,
}
