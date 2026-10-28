//! Vehicle bundles of the Chameleon engine. Spec: `docs/formats/chameleon-vehicle.md`.
//!
//! A vehicle bundle holds one `VehicleGraphicsSpec` (the body model and four wheel placements), the models and
//! renderables they use, materials, a few textures, and gameplay objects. [`load`] walks from the spec to the geometry
//! and gives each mesh a reduced [`Appearance`]. Shaders, vertex layouts and most textures live in shared bundles, so
//! the caller puts those in the same [`chameleon_bundle::Pool`].

mod active_aero;
mod appearance;
mod arch;
mod body_finish;
mod collision;
mod colour_palette;
mod corona_records;
mod damage_clip;
mod drift;
mod drift_model;
mod driver;
mod driver_cop;
mod driver_rig;
mod error;
mod genesys;
mod hinged;
mod hsm;
mod lamp_pieces;
mod lamps;
mod manufacturer;
mod name;
mod paint;
mod paint_pack;
mod paint_surface;
mod plate;
mod plate_design;
mod police;
mod skeleton;
mod skin;
mod slide;
mod spec;
mod stats;
mod steering_wheel;
mod traffic;
mod two_colour;
mod vehicle;
mod wing;

#[cfg(test)]
mod damage_clip_tests;
#[cfg(test)]
mod drift_tests;
#[cfg(test)]
mod driver_rig_tests;
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
pub use appearance::{Appearance, LampCircuit, LampEnd, LampSide, LampSurface, SurfaceKind};
pub use arch::{TyreVolume, intrusion, prefer_other};
pub use body_finish::{PLAIN as PLAIN_BODY_TECHNIQUE, PlainSurface, plain_surfaces};
pub use collision::{PolySoup, load_poly_soup};
pub use colour_palette::{ColourGroup, TwoColourPack, colour_groups};
pub use corona_records::{CoronaRecord, read_corona_records};
pub use damage_clip::{BREAK_STEP_M, BonePose, BoneTrack, DamageClip, damage_clips};
pub use drift::{DriftNitrous, DriftPayout, DriftPhysics, TANK_UNITS, read_drift_nitrous, read_drift_physics};
pub use drift_model::{DriftInput, DriftModel, DriftOutput, DriftParams, DriftSummary, HIT_ENDS};
pub use driver::{Driver, DriverSetup, driver_meshes, driver_setup, is_driver, load_driver, remove_baked_driver};
pub use driver_cop::COP_CHARACTER;
pub use driver_rig::{DriverPose, DriverRig, Grips, LOOK_BACK_CLIP, STEERING_CLIP, driver_grips, load_rig};
pub use error::{Error, Result};
pub use hinged::{HingedPart, hinged_parts};
pub use hsm::{DriveSpec, Guard, Threshold};
pub use lamps::{LampEntry, Lamps, Locator, LocatorGroup, SpotEffect, read_groups, read_lamps, split_lenses};
pub use manufacturer::{Manufacturer, localised_text, manufacturers, parse_manufacturers};
pub use name::vehicle_name;
pub use paint::{VehiclePaint, paints};
pub use paint_pack::{PaintPack, PaintPackGroup, paint_packs};
pub use paint_surface::{PaintBlend, PaintSurface, paint_surfaces};
pub use plate::{
    BLANK as PLATE_BLANK, COLUMNS as PLATE_COLUMNS, GLYPHS as PLATE_GLYPHS, GamePack, ROWS as PLATE_ROWS,
    SLOTS as PLATE_SLOTS, cells as plate_cells, default_for as plate_default_for, default_texts as plate_default_texts,
    game_packs, is_plate_technique, registration as plate_registration,
};
pub use plate_design::{
    Area as PlateArea, DESIGN_PICTURES as PLATE_DESIGN_PICTURES, PlateDesign, STRIP_ASPECT as PLATE_STRIP_ASPECT,
    design_band as plate_design_band, design_picture as plate_design_picture,
    design_picture_path as plate_design_picture_path, design_plate as plate_design_plate, plate_designs,
};
pub use police::{PoliceMarks, police_marks};
pub use skeleton::{Bone, Skeleton, bone_hash};
pub use skin::{BodyParts, body_parts};
pub use slide::{DriftHandling, read_drift_handling};
pub use spec::{GraphicsSpec, WheelSpec, find_spec};
pub use stats::{TorqueKnot, VehicleStats, WheelStats, read_stats};
pub use steering_wheel::{BONE as STEERING_BONE, DEFAULT_LOCK_DEGREES, WheelFrame, lock_degrees, wheel_frame};
pub use traffic::{TrafficName, traffic_name};
pub use two_colour::{Mask, Sheet};
pub use vehicle::{Lod, PartMesh, Vehicle, Wheel, load, load_texture};
pub use wing::{WingBoneTrack, WingMachine, WingParams, wing_machine};
