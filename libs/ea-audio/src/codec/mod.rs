//! The sample codecs.
//!
//! - [`xa`]: EA-XA ADPCM (28 samples per frame), the default on PC.
//! - [`xas`]: EA-XAS version 0 (32 samples per 0x13-byte frame), used by granular engine loops.
//! - [`microtalk`]: EA MicroTalk 10:1 (UTK), a speech codec.

pub mod microtalk;
pub mod xa;
pub mod xas;
