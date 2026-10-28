//! Skeletal animation of the Chameleon engine (NFS: Most Wanted 2012). Spec:
//! `docs/formats/chameleon-driver-animation.md`.
//!
//! A character is a skinned model, a [`Skeleton`] and an [`AnimationList`] of [`Clip`]s. This crate reads the skeleton and the
//! clips and turns a pose (a rotation per bone) into the matrices that skin a model. It takes `&[u8]` and never opens files.

mod bytes;
mod clip;
mod error;
pub mod ik;
mod list;
mod pose;
pub mod quat;
mod skeleton;

#[cfg(test)]
mod clip_tests;
#[cfg(test)]
mod install_tests;

pub use clip::Clip;
pub use error::{Error, Result};
pub use list::AnimationList;
pub use pose::{Deltas, Pose, Skin};
pub use skeleton::{Bone, Skeleton, bone_hash};
