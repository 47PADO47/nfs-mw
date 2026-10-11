//! Errors of the upscaler context.

use thiserror::Error;
use wgpu::TextureFormat;

/// Why a context could not be created or a dispatch could not be recorded.
#[derive(Debug, Error)]
pub enum Fsr3Error {
    /// The render or output size is smaller than 2x2.
    #[error("the {what} size {width}x{height} is below the 2x2 minimum")]
    SizeTooSmall {
        /// Which size: "render" or "output".
        what: &'static str,
        /// The width given.
        width: u32,
        /// The height given.
        height: u32,
    },
    /// The render size is larger than the output size, which the upscaler cannot do.
    #[error("the render size {render:?} is larger than the output size {output:?}")]
    RenderLargerThanOutput {
        /// The render size given.
        render: [u32; 2],
        /// The output size given.
        output: [u32; 2],
    },
    /// The format of the output texture cannot be written as a storage texture in WGSL.
    #[error("{0:?} cannot be used as the upscaler's output: use Rgba16Float, Rgba32Float or Rgba8Unorm")]
    UnsupportedOutputFormat(TextureFormat),
    /// The camera parameters cannot describe a projection.
    #[error("the camera parameters are invalid: {0}")]
    InvalidCamera(&'static str),
}
