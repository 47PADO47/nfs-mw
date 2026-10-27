//! Graphics resources of the Chameleon engine. Spec: `docs/formats/chameleon-gfx.md`.
//!
//! Every decoder takes the blocks of one resource (see `chameleon-bundle`) and the import table, and never opens files.
//! Resources point at each other by id; resolving an id to its bytes is the caller's job (`chameleon-vehicle` does it
//! for cars).

mod bytes;
mod dds;
#[cfg(test)]
mod dds_tests;
mod error;
mod material;
mod model;
mod renderable;
mod rgba;
mod shader;
mod texture;
mod vertex;

pub use error::{Error, Result};
pub use material::Material;
pub use model::Model;
pub use renderable::{Mesh, Primitive, Renderable};
pub use shader::Shader;
pub use texture::{PixelFormat, Texture, mip_size};
pub use vertex::{Element, POSITION_SCALE, Vertex, VertexDescriptor, usage};
