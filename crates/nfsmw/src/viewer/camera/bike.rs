//! The cameras of a two-wheeler (`docs/specs/ride6-bike-cameras.md`). The game's cameras are made for a car: the views of its
//! `camerainfo` data sit at car heights and distances, and the custom chase camera is car-sized. A bike carries its own camera set
//! (`nfsmw_data::car::mods::bike_camera`, the `[bike.camera]` table of its manifest); this module turns it into
//!
//! - the seven views of the original camera ([`views`]), whose `close` and `far` are the bike's near and far chase and whose
//!   other chase views keep the car's proportions to `far`;
//! - the fit of the custom chase camera ([`chase_fit`]);
//! - the two cameras mounted on the bike, the one ahead of it and the one in the rider's helmet ([`lens`]).
//!
//! Space is the world's (x forward at heading 0, z up); a camera set is in the bike's model frame, x forward, y left, z up.

use glam::{Quat, Vec3};
use nfsmw_data::camera::{View, Views};
use nfsmw_data::car::mods::bike_camera::{BikeCamera, Chase, Front, Helmet};

use super::chase::{Fit, REFERENCE_DISTANCE, REFERENCE_HEIGHT};
use super::{Lens, direction};

/// The aspect the camera set's fields of view are given at (a 16:9 screen); the vertical field of view of a mounted camera is
/// fixed from it, so a wider screen shows more at the sides.
pub const REFERENCE_ASPECT: f32 = 16.0 / 9.0;
/// The aspect the original camera's field-of-view numbers are given at: its optics take them as the horizontal field of view of
/// a 4:3 screen (`projection.rs`).
const ORIGINAL_ASPECT: f32 = 4.0 / 3.0;

/// The vertical field of view, degrees, of a camera whose horizontal field of view at [`REFERENCE_ASPECT`] is `horizontal`.
pub fn vertical_fov(horizontal: f32) -> f32 {
    2.0 * ((horizontal.to_radians() * 0.5).tan() / REFERENCE_ASPECT).atan().to_degrees()
}

/// The number the original camera's views hold for a horizontal field of view given at [`REFERENCE_ASPECT`].
fn original_fov(horizontal: f32) -> f32 {
    let tangent = (horizontal.to_radians() * 0.5).tan() * ORIGINAL_ASPECT / REFERENCE_ASPECT;
    2.0 * tangent.atan().to_degrees()
}

/// The fit of the custom chase camera to a bike: its rest distance and height are the near chase's, and its look point the near
/// chase's.
pub fn chase_fit(rig: &BikeCamera) -> Fit {
    let near = &rig.chase_near;
    Fit {
        distance: near.distance / REFERENCE_DISTANCE,
        height: near.height / REFERENCE_HEIGHT,
        look_height: near.target_height,
    }
}

/// The original camera's seven views for a bike, from the car views `base`. `anchor_height` is how high above the ground the
/// camera's anchor (the bike's centre of gravity) is. `close` and `far` are the bike's chase views; `superfar`, `drift` and
/// `pursuit` are `far` scaled by the proportions the car's own views have to its `far` (so the order of the views is kept); the
/// two views inside the bike (`bumper`, `hood`) are not used (the cameras mounted on the bike replace them, see [`lens`]) and
/// hold the near chase.
pub fn views(rig: &BikeCamera, anchor_height: f32, base: &Views) -> Views {
    let reference = &base.views[3];
    let ratio = |from: f32, to: f32| if to.abs() < 1e-3 { 1.0 } else { from / to };
    let scaled = |from: &View| {
        let lag = rig.chase_far.distance * ratio(from.lag[0], reference.lag[0]);
        let height = rig.chase_far.height * ratio(from.height[0], reference.height[0]);
        (lag, height)
    };
    let mut out = base.clone();
    for (index, view) in base.views.iter().enumerate() {
        let (chase, lag, height) = match index {
            0..=2 => (&rig.chase_near, rig.chase_near.distance, rig.chase_near.height),
            3 => (&rig.chase_far, rig.chase_far.distance, rig.chase_far.height),
            _ => {
                let (lag, height) = scaled(view);
                (&rig.chase_far, lag, height)
            }
        };
        out.views[index] = chase_view(view, chase, (lag, height), anchor_height);
    }
    out
}

/// One chase view: `base`'s controller settings with the camera `lag` behind and `height` above the ground, looking at `chase`'s
/// point.
fn chase_view(base: &View, chase: &Chase, (lag, height): (f32, f32), anchor_height: f32) -> View {
    // The original controller looks at the point straight above its anchor; the angle makes that look along the line to
    // the chase's own look point.
    let drop = height - chase.target_height;
    let angle = (drop / (lag + chase.target_ahead).max(0.1)).atan().to_degrees();
    View {
        fov: [original_fov(chase.fov); 2],
        height: [height - anchor_height; 2],
        lag: [-lag; 2],
        angle: [angle; 2],
        lat_offset: [0.0; 2],
        ..*base
    }
}

/// The attitude of a bike: its heading, its pitch (nose up positive) and its roll (leaning right positive), radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attitude {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

impl Attitude {
    /// The attitude of a model frame turned by `rotation` (x forward, y left, z up).
    pub fn of(rotation: Quat) -> Self {
        let (forward, left, up) = (rotation * Vec3::X, rotation * Vec3::Y, rotation * Vec3::Z);
        Self { yaw: forward.y.atan2(forward.x), pitch: forward.z.clamp(-1.0, 1.0).asin(), roll: left.z.atan2(up.z) }
    }
}

/// A camera fixed to the bike: how it is turned from the bike's axis and how much of the bike's motion it takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mount {
    /// Looks back along the bike rather than ahead.
    pub look_back: bool,
    pub pitch_deg: f32,
    pub fov: f32,
    pub near: f32,
    pub pitch_factor: f32,
    pub roll_factor: f32,
    /// The roll followed beyond `roll_factor`, degrees, up to this much.
    pub follow_roll_max_deg: f32,
}

impl Mount {
    /// The camera ahead of the bike; `look_back` flips its direction (the look-back control).
    pub fn front(front: &Front, look_back: bool) -> Self {
        Self {
            look_back: front.look_back != look_back,
            pitch_deg: front.pitch_deg,
            fov: front.fov,
            near: front.near,
            pitch_factor: front.pitch_factor,
            roll_factor: front.roll_factor,
            follow_roll_max_deg: 0.0,
        }
    }

    /// The camera in the helmet; the look-back control turns it to look over the tail.
    pub fn helmet(helmet: &Helmet, look_back: bool) -> Self {
        Self {
            look_back,
            pitch_deg: helmet.pitch_deg,
            fov: helmet.fov,
            near: helmet.near,
            pitch_factor: helmet.pitch_factor,
            roll_factor: helmet.roll_factor,
            follow_roll_max_deg: helmet.follow_roll_max_deg,
        }
    }

    /// How far the camera rolls for a bike rolled by `roll` radians: its share of it, and the bike's roll up to the follow limit.
    pub fn roll(&self, roll: f32) -> f32 {
        let own = roll * self.roll_factor;
        let limit = self.follow_roll_max_deg.to_radians();
        own + (roll * (1.0 - self.roll_factor)).clamp(-limit, limit)
    }
}

/// The lens of a camera `mount`ed on a bike with attitude `bike`, at the world point `eye`.
pub fn lens(eye: Vec3, bike: Attitude, mount: &Mount, aspect: f32) -> Lens {
    // A camera bolted to the bike that looks back is tilted the other way by the bike's pitch.
    let (turn, tilt) = match mount.look_back {
        true => (std::f32::consts::PI, -bike.pitch),
        false => (0.0, bike.pitch),
    };
    let forward = direction(bike.yaw + turn, mount.pitch_deg.to_radians() + mount.pitch_factor * tilt);
    // The roll turns the world's up about the bike's heading: a bike leaning right tips the camera's up to the right.
    let up = Quat::from_axis_angle(direction(bike.yaw, 0.0), mount.roll(bike.roll)) * Vec3::Z;
    let mut lens = Lens::look_at(eye, eye + forward, vertical_fov(mount.fov), mount.near, aspect);
    lens.view = glam::camera::rh::view::look_at_mat4(eye, eye + forward, up);
    lens
}

#[cfg(test)]
#[path = "bike_tests.rs"]
mod tests;
