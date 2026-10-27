//! Gameplay cameras of the Chameleon engine (NFS: Most Wanted 2012). Spec: `docs/formats/chameleon-camera.md`.
//!
//! Every car bundle carries a `VehicleCameraContainer`: the data of the car's chase ("external"), bonnet, bumper and
//! rear-view cameras plus a few shared globals (impact shake, lens, gradient adjustments). [`read_cameras`] decodes
//! it into [`CarCameras`] (host space: x forward, y left, z up, metres, degrees, seconds). [`CameraState`] is a
//! behaviour model the host steps once per rendered frame with a [`CarMotion`] to get a [`CameraPose`].
//!
//! The data layout and its per-car values are **[verified]** on the 93 cars of the install; the behaviour of the model
//! is built from the data's structure names and values, not from the executable, so it is **[unconfirmed]** (the doc
//! lists each reading). This crate takes a parsed [`chameleon_bundle::Bundle`] and never opens files.

mod error;
mod genesys;
mod params;
mod read;

pub use error::{Error, Result};
pub use params::{
    AccelMovement, CameraRoll, CarCameras, DecelPitch, ExternalCamera, ExternalGlobals, GradientAdjustments,
    HighSpeedShake, ImpactShake, Lens, MountedCamera, RearView, SpeedMovement, Vec3, YawSpring, from_data_axes,
};
pub use read::{read_all_cameras, read_cameras};
