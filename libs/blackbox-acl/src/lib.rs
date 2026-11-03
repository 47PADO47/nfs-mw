// The layout, the bit packing, the range expansion and the interpolation of this decoder follow the Animation Compression
// Library (https://github.com/nfrechette/acl), rewritten in Rust. Its licence:
//
// The MIT License (MIT)
//
// Copyright (c) 2017 Nicholas Frechette & Animation Compression Library contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit
// persons to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
// Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
// WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! A decoder of [ACL](https://github.com/nfrechette/acl) (Animation Compression Library, MIT) compressed transform
//! tracks, the codec Unreal Engine's ACL plugin stores skeletal animations in.
//!
//! Bytes in, one rotation, translation and scale per track and frame out, in pure Rust. The crate reads buffers of
//! revision 9 and 10 (ACL 2.1): the uniformly sampled algorithm with full or quantised (variable bit rate)
//! rotations, translations and scales, default and constant sub-tracks, range reduction by clip and by segment,
//! stripped keyframes and looping clips. It takes `&[u8]` and never opens files.
//!
//! ```no_run
//! use blackbox_acl::{Qvv, Tracks};
//! # fn demo(bytes: &[u8], reference_pose: &[Qvv]) -> Result<(), blackbox_acl::AclError> {
//! let tracks = Tracks::parse(bytes)?;
//! let frames = tracks.decode_all(Some(reference_pose))?; // frames[frame][track]
//! let halfway = tracks.sample(tracks.duration() * 0.5, Some(reference_pose))?;
//! # let _ = (frames, halfway);
//! # Ok(()) }
//! ```
//!
//! The format is described in `docs/specs/acl-decoder.md`.

mod bits;
mod decode;
mod error;
mod header;
mod layout;
mod qvv;
mod tracks;

#[cfg(test)]
mod encoder;
#[cfg(test)]
mod real_data;
#[cfg(test)]
mod tests;

pub use error::AclError;
pub use header::{RotationFormat, VectorFormat};
pub use qvv::Qvv;
pub use tracks::Tracks;
