//! Vehicle bundles of the Chameleon engine. Spec: `docs/formats/chameleon-vehicle.md`.
//!
//! A vehicle bundle holds one `VehicleGraphicsSpec` (the body model and four wheel placements), the models and
//! renderables they use, materials, a few textures, and gameplay objects. [`load`] walks from the spec to the geometry
//! and gives each mesh a reduced [`Appearance`]. Shaders, vertex layouts and most textures live in shared bundles, so
//! the caller puts those in the same [`chameleon_bundle::Pool`].

mod active_aero;
mod appearance;
mod arch;
mod collision;
mod colour_palette;
mod driver;
mod error;
mod genesys;
mod hinged;
mod hsm;
mod lamps;
mod manufacturer;
mod name;
mod paint;
mod paint_pack;
mod paint_surface;
mod police;
mod skeleton;
mod skin;
mod spec;
mod stats;
mod two_colour;
mod vehicle;
mod wing;

#[cfg(test)]
mod driver_tests;
#[cfg(test)]
mod stats_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod two_colour_tests;

pub use active_aero::{
    AttachedMotion, FlapMotion, SpoilerMotion, VentMotion, attached_motions, flap_motions, spoiler_motion, vent_motions,
};
pub use appearance::{Appearance, LampEnd, LampSide, LampSurface, SurfaceKind};
pub use arch::{TyreVolume, intrusion, prefer_other};
pub use collision::{PolySoup, load_poly_soup};
pub use colour_palette::{ColourGroup, TwoColourPack, colour_groups};
pub use driver::{Driver, DriverSetup, driver_meshes, driver_setup, is_driver, load_driver, remove_baked_driver};
pub use error::{Error, Result};
pub use hinged::{HingedPart, hinged_parts};
pub use hsm::{DriveSpec, Guard, Threshold};
pub use lamps::{LampEntry, Lamps, Locator, LocatorGroup, SpotEffect, read_groups, read_lamps, split_lenses};
pub use manufacturer::{Manufacturer, localised_text, manufacturers, parse_manufacturers};
pub use name::vehicle_name;
pub use paint::{VehiclePaint, paints};
pub use paint_pack::{PaintPack, PaintPackGroup, paint_packs};
pub use paint_surface::{PaintBlend, PaintSurface, paint_surfaces};
pub use police::{PoliceMarks, police_marks};
pub use skeleton::{Bone, Skeleton, bone_hash};
pub use skin::{BodyParts, body_parts};
pub use spec::{GraphicsSpec, WheelSpec, find_spec};
pub use stats::{TorqueKnot, VehicleStats, WheelStats, read_stats};
pub use two_colour::{Mask, Sheet};
pub use vehicle::{Lod, PartMesh, Vehicle, Wheel, load, load_texture};
pub use wing::{WingBoneTrack, WingMachine, WingParams, wing_machine};
