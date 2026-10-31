//! Skeletal animation runtime: a bone hierarchy, sampled clips, pose blending, blend trees as data, two-bone IK and
//! linear-blend skinning on the CPU.
//!
//! The crate knows nothing of any game or file format of a game. Everything is plain structs and slices; the only
//! file format is the crate's own [`pack::Pack`], which converters write. Maths are `f32`, right-handed or
//! left-handed as the data is: the crate never assumes a handedness, an up axis or a unit.
//!
//! - [`transform`], [`skeleton`], [`pose`]: the data model and pose arithmetic (blend, masked blend, additive).
//! - [`clip`]: per-bone rotation, translation and scale channels (constant, evenly sampled or keyed), sampling with
//!   interpolation, looping, notifies, conversion to additive.
//! - [`blend_space`], [`graph`]: a blend tree (`Clip`, `Blend`, `ThreeWay`, `Additive`, `Layer`, `Space`, `Select`)
//!   whose weights are named parameters, evaluated into a pose.
//! - [`ik`]: two-bone IK with a pole.
//! - [`fall`]: the animation of a character thrown to the ground (launch, tumble and get-up trees and the state machine that
//!   plays them).
//! - [`skin`]: skinning matrices and CPU linear-blend skinning of vertex arrays.
//! - [`pack`]: the binary container of a character (skeleton, sockets, clips, skinned meshes).
//!
//! The behaviour is specified in `docs/specs/skeletal-animation.md`.

pub mod blend_space;
mod bytes;
pub mod clip;
mod clip_io;
mod error;
pub mod fall;
pub mod graph;
pub mod ik;
pub mod pack;
pub mod pose;
pub mod skeleton;
pub mod skin;
pub mod transform;

pub use blend_space::{Axis, BlendSpace, SpaceSample};
pub use clip::{Channel, Clip, Notify, Track};
pub use error::{Error, Result};
pub use fall::{FallInput, FallPlayer, FallSet};
pub use graph::{EvalContext, Graph, Node, TimeMode, Value};
pub use ik::{TwoBoneChain, apply_two_bone};
pub use pack::{MeshLod, Pack, Section, SkinnedMesh, Socket};
pub use pose::Pose;
pub use skeleton::{Bone, Skeleton};
pub use skin::{Influences, MAX_INFLUENCES, SkinVertex, inverse_bind_from_rest, skin_matrices, skin_vertices};
pub use transform::Transform;
