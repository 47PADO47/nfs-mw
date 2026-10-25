//! Vehicle bundles of the Chameleon engine. Spec: `docs/formats/chameleon-vehicle.md`.
//!
//! A vehicle bundle holds one `VehicleGraphicsSpec` (the body model and four wheel placements), the models and
//! renderables they use, materials, a few textures, and gameplay objects. [`load`] walks from the spec to the geometry
//! and gives each mesh a reduced [`Appearance`]. Shaders, vertex layouts and most textures live in shared bundles, so
//! the caller puts those in the same [`chameleon_bundle::Pool`].

mod appearance;
mod error;
mod name;
mod spec;
mod vehicle;

#[cfg(test)]
mod tests;

pub use appearance::{Appearance, SurfaceKind};
pub use error::{Error, Result};
pub use name::vehicle_name;
pub use spec::{GraphicsSpec, WheelSpec, find_spec};
pub use vehicle::{Lod, PartMesh, Vehicle, Wheel, load, load_texture};
