//! What a viewer scene is, and the cameras scenes share. The window, loop and renderer belong to
//! [`crate::app`]; input arrives as actions ([`crate::input`]).

pub mod camera;
mod race_report;
mod static_hash;
mod vehicle;
pub use race_report::{
    MILESTONE_LINES, MilestoneLine, MilestoneStatus, PhotoKind, PhotoReport, PursuitPage, PursuitReport, RaceAnswer,
    RaceReport, ReportLap, ReportRacer, RewardLine, RewardsReport, photo_picture,
};
pub use static_hash::StaticHash;
pub use vehicle::VehicleScene;

mod scene;
pub use scene::Scene;

mod fullscreen;
pub use fullscreen::{Fit, Fullscreen};

mod environment;
pub use environment::{Environment, SunMode};

mod readouts;
pub use readouts::{CameraReadout, PlayerCar, WorldInfo};
