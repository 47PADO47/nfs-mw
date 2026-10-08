//! Video frames: the planar YUV picture and its RGBA conversion, and (with the `vp6` feature) the
//! VP6 decoder.

mod yuv;
pub use yuv::YuvFrame;

#[cfg(feature = "vp6")]
mod vp6;
#[cfg(feature = "vp6")]
pub use vp6::Vp6Decoder;
