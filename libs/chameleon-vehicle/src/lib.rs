//! Vehicle bundles of the Chameleon engine. Spec: `docs/formats/chameleon-vehicle.md`.
//!
//! A vehicle bundle holds one `VehicleGraphicsSpec` (the body model and four wheel placements), the models and
//! renderables they use, materials, a few textures, and gameplay objects. [`load`] walks from the spec to the geometry
//! and gives each mesh a reduced [`Appearance`]. Shaders, vertex layouts and most textures live in shared bundles, so
//! the caller puts those in the same [`chameleon_bundle::Pool`].

mod active_aero;
mod appearance;
mod collision;
mod error;
mod genesys;
mod hinged;
mod hsm;
mod lamps;
mod name;
mod paint;
mod skeleton;
mod skin;
mod spec;
mod stats;
mod vehicle;
mod wing;

#[cfg(test)]
mod stats_tests;
#[cfg(test)]
mod tests;

pub use active_aero::{AttachedMotion, SpoilerMotion, VentMotion, attached_motions, spoiler_motion, vent_motions};
pub use appearance::{Appearance, LampEnd, LampSide, LampSurface, SurfaceKind};
pub use collision::{PolySoup, load_poly_soup};
pub use error::{Error, Result};
pub use hinged::{HingedPart, hinged_parts};
pub use lamps::{LampEntry, Lamps, Locator, LocatorGroup, SpotEffect, read_groups, read_lamps, split_lenses};
pub use name::vehicle_name;
pub use paint::{VehiclePaint, paints};
pub use skeleton::{Bone, Skeleton, bone_hash};
pub use skin::{BodyParts, body_parts};
pub use spec::{GraphicsSpec, WheelSpec, find_spec};
pub use stats::{TorqueKnot, VehicleStats, WheelStats, read_stats};
pub use vehicle::{Lod, PartMesh, Vehicle, Wheel, load, load_texture};
pub use wing::{
    BRAKE_HOLD_SECONDS as WING_BRAKE_HOLD_SECONDS, BRAKE_MPH as WING_BRAKE_MPH, FADE_SECONDS as WING_FADE_SECONDS,
    OPEN_MPH as WING_OPEN_MPH, TOP_MPH as WING_TOP_MPH, WingBoneTrack, WingMachine, wing_machine,
};
