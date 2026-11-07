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
use nfsmw_data::car::mods::bike_camera::{BikeCamera, Chase, Front, Helmet, Lean};

use super::chase::{Fit, REFERENCE_DISTANCE, REFERENCE_HEIGHT};
use super::dynamic_fov::{REST_FOV_DEGREES, TOP_FOV_DEGREES, scaled_fov_for_speed};
use super::head_look::HeadLook;
use super::{Lens, direction};

/// How far the helmet camera is tipped down on top of the data's own tilt (`RelativeRotation.Pitch`, -10 degrees on 312 of the 344
/// bikes), degrees. A modelling choice that is a player's request, not RIDE 6's number: with the data's tilt alone the helmet's
/// straight view is mostly road and sky and the bike barely shows at the bottom of the picture. With [`HELMET_FOV_DEG`] this
/// puts the bottom edge of the picture 60 degrees below the horizon (the grips of the 277 imported bikes are 20 to 60 degrees below
/// the rider's eyes, 48 on median) and still leaves the horizon in the upper 30 percent of the frame
/// (`docs/specs/ride6-bike-cameras.md`, section 5). The data has no larger real number to use for it: the 50 and 60 degrees of
/// two supermotard bikes are capped (`MAX_HELMET_DOWN_DEG` of the importer) and the others range from 5 to 20.
pub const HELMET_VIEW_DOWN_DEG: f32 = 16.0;

/// The vertical field of view of the helmet camera at [`REFERENCE_ASPECT`], degrees. A modelling choice that is a player's request
/// ("more field of view, I should see the bike's display normally"), wider than RIDE 6's own 91.49 degrees horizontal (60
/// vertical) which cut the handlebar and the instrument panel off at the picture's lower and side edges: from the rider's eyes the
/// grips of the imported bikes are 33 to 65 degrees to the side (45 on median, 52 at the 90th percentile) and 20 to 60 degrees
/// down, so the picture must reach about 55 degrees to either side and, with [`HELMET_VIEW_DOWN_DEG`] and the data's 10 degrees
/// of tilt, 60 degrees down. 88 degrees vertical is 120 horizontal at 16:9 (the user asked for a wider picture that looks lower: 80 and 10 before). A bike whose data field of view is wider keeps it.
pub const HELMET_FOV_DEG: f32 = 88.0;

/// How many degrees of vertical field of view the helmet camera gains at full speed on top of [`HELMET_FOV_DEG`] (scaled by the
/// speed curve of the dynamic field of view and by the player's field-of-view intensity). Only ever widens, so the instrument
/// panel stays in the picture at every speed.
pub const HELMET_SPEED_WIDEN_DEG: f32 = 8.0;

/// The widest vertical field of view a mounted camera has, degrees: the floor that keeps the helmet camera's sides on a narrow
/// picture (a split screen) stops here.
const MAX_FOV_DEG: f32 = 110.0;

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

/// The horizontal field of view at [`REFERENCE_ASPECT`], degrees, of a camera whose vertical field of view is `vertical`.
pub fn horizontal_fov(vertical: f32) -> f32 {
    2.0 * ((vertical.to_radians() * 0.5).tan() * REFERENCE_ASPECT).atan().to_degrees()
}

/// The vertical field of view, degrees, that gives `horizontal` degrees across a picture of `aspect` (width over height).
fn vertical_fov_for(horizontal: f32, aspect: f32) -> f32 {
    2.0 * ((horizontal.to_radians() * 0.5).tan() / aspect.max(0.1)).atan().to_degrees()
}

/// How much the helmet camera's vertical field of view widens at `speed` (m/s, either direction), degrees: the speed curve of the
/// dynamic field of view (nothing at rest, all of [`HELMET_SPEED_WIDEN_DEG`] at its top speed) scaled by `intensity` (0 to 1).
pub fn speed_widening(speed: f32, intensity: f32) -> f32 {
    let share = (scaled_fov_for_speed(speed, intensity) - REST_FOV_DEGREES) / (TOP_FOV_DEGREES - REST_FOV_DEGREES);
    HELMET_SPEED_WIDEN_DEG * share.clamp(0.0, 1.0)
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
    /// The horizontal field of view at [`REFERENCE_ASPECT`], degrees.
    pub fov: f32,
    /// The least horizontal field of view the camera keeps on a narrower picture than [`REFERENCE_ASPECT`], degrees (0: none, the
    /// camera's vertical field of view is kept and a narrow picture shows less at the sides).
    pub min_horizontal_deg: f32,
    /// Vertical degrees added to the field of view (the helmet camera's speed widening, [`speed_widening`]).
    pub widen_deg: f32,
    pub near: f32,
    pub pitch_factor: f32,
    pub roll_factor: f32,
    /// The roll followed beyond `roll_factor`, degrees, up to this much.
    pub follow_roll_max_deg: f32,
    /// Whether the camera's place is carried by the bike's heading and pitch and only the roll it follows (a camera on a pivot
    /// that does not lean with the bike, [`Mount::place`]) rather than bolted to the leaning body.
    pub level_position: bool,
    /// What the camera does as the bike leans into a bend ([`Lean::NONE`]: nothing).
    pub lean: Lean,
    /// Where the rider's head is turned from straight ahead by the player's look stick ([`HeadLook`]; straight ahead for a camera
    /// that has none).
    pub look: HeadLook,
}

impl Mount {
    /// The camera ahead of the bike; `look_back` flips its direction (the look-back control). It is bolted to the body.
    pub fn front(front: &Front, look_back: bool) -> Self {
        Self {
            look_back: front.look_back != look_back,
            pitch_deg: front.pitch_deg,
            fov: front.fov,
            min_horizontal_deg: 0.0,
            widen_deg: 0.0,
            near: front.near,
            pitch_factor: front.pitch_factor,
            roll_factor: front.roll_factor,
            follow_roll_max_deg: 0.0,
            level_position: false,
            lean: Lean::NONE,
            look: HeadLook::default(),
        }
    }

    /// The camera in the helmet; the look-back control turns it to look over the tail. It stays upright over the bike's origin
    /// as the bike leans ([`Mount::place`]), looks [`HELMET_VIEW_DOWN_DEG`] further down than the data has it and sees at least
    /// [`HELMET_FOV_DEG`] vertically (the instrument panel and the handlebar stay in the picture, also on a narrow one).
    pub fn helmet(helmet: &Helmet, look_back: bool) -> Self {
        Self {
            look_back,
            pitch_deg: helmet.pitch_deg - HELMET_VIEW_DOWN_DEG,
            fov: helmet.fov.max(horizontal_fov(HELMET_FOV_DEG)),
            min_horizontal_deg: horizontal_fov(HELMET_FOV_DEG),
            widen_deg: 0.0,
            near: helmet.near,
            pitch_factor: helmet.pitch_factor,
            roll_factor: helmet.roll_factor,
            follow_roll_max_deg: helmet.follow_roll_max_deg,
            level_position: true,
            lean: helmet.lean,
            look: HeadLook::default(),
        }
    }

    /// The same camera with the rider's head turned by `look` (the helmet camera only: the other cameras of a bike have no head).
    pub fn with_head(self, look: HeadLook) -> Self {
        Self { look, ..self }
    }

    /// The same camera with its field of view widened by `degrees` (vertical; [`speed_widening`]).
    pub fn with_widening(self, degrees: f32) -> Self {
        Self { widen_deg: degrees, ..self }
    }

    /// The vertical field of view of the picture, degrees, at `aspect` (width over height): the camera's own plus its widening,
    /// and wide enough across for [`Mount::min_horizontal_deg`] on a picture narrower than the reference one.
    pub fn fov_y(&self, aspect: f32) -> f32 {
        let own = vertical_fov(self.fov) + self.widen_deg;
        let floor = match self.min_horizontal_deg > 0.0 {
            true => vertical_fov_for(self.min_horizontal_deg, aspect),
            false => 0.0,
        };
        own.max(floor).min(MAX_FOV_DEG.max(own))
    }

    /// How far the camera rolls for a bike rolled by `roll` radians: its share of it, and the bike's roll up to the follow limit.
    pub fn roll(&self, roll: f32) -> f32 {
        let own = roll * self.roll_factor;
        let limit = self.follow_roll_max_deg.to_radians();
        own + (roll * (1.0 - self.roll_factor)).clamp(-limit, limit)
    }

    /// The world place of the point `in_model` of the bike's model frame (whose origin is `origin` and whose attitude is
    /// `rotation`) as this camera has it. A bolted camera has it where the body carries it. A helmet camera has it carried by the
    /// bike's heading and pitch and only the roll it follows ([`Mount::roll`]), and moved by the lean's shift: its point is
    /// not on the body that leans over, it is the rider's head, which stays up out of the bend (RIDE 6's helmet camera has a
    /// `RollFactor` of 0 and a follow limit of 5 degrees: it does not roll the camera's mount with the bike). Rigidly
    /// following the body put the eye of a bike at 60 degrees of lean 70 cm sideways and half its height, a view of the road
    /// alone.
    pub fn place(&self, origin: Vec3, rotation: Quat, in_model: Vec3) -> Vec3 {
        if !self.level_position {
            return origin + rotation * in_model;
        }
        let roll = Attitude::of(rotation).roll;
        // The body's frame with the roll the camera does not follow taken out, about the bike's own forward axis.
        let upright = rotation * Quat::from_rotation_x(self.roll(roll) - roll);
        origin + upright * (in_model + self.lean_shift(roll))
    }

    /// The shift of the camera's point by the lean of a bike rolled by `roll` radians (right positive), in the model's frame.
    pub fn lean_shift(&self, roll: f32) -> Vec3 {
        let share = self.lean.shift_share(roll.to_degrees());
        let [ahead, inward, up] = self.lean.shift_m;
        // The model's y is to the left: the inside of a lean to the right is -y.
        Vec3::new(ahead, -inward * side(roll), up) * share
    }
}

/// +1 for a bike rolled to the right, -1 to the left.
fn side(roll: f32) -> f32 {
    1.0_f32.copysign(roll)
}

/// The lens of a camera `mount`ed on a bike with attitude `bike`, at the world point `eye` (see [`Mount::place`]).
pub fn lens(eye: Vec3, bike: Attitude, mount: &Mount, aspect: f32) -> Lens {
    // A camera bolted to the bike that looks back is tilted the other way by the bike's pitch.
    let (turn, tilt) = match mount.look_back {
        true => (std::f32::consts::PI, -bike.pitch),
        false => (0.0, bike.pitch),
    };
    // The lean turns the look into the bend and tips it down a little (the way back is the way the bike came: not turned).
    let lean_deg = bike.roll.to_degrees();
    let (into_bend, down) = match mount.look_back {
        true => (0.0, 0.0),
        false => (
            -side(bike.roll) * mount.lean.turn_yaw_deg.to_radians() * mount.lean.turn_share(lean_deg),
            mount.lean.shift_pitch_deg.to_radians() * mount.lean.shift_share(lean_deg),
        ),
    };
    // The head's look is on top of all of it: a stick to the right turns the view clockwise whichever way it faces.
    let (head_right, head_up) = (mount.look.right_deg().to_radians(), mount.look.up_deg().to_radians());
    let forward = direction(
        bike.yaw + turn + into_bend - head_right,
        mount.pitch_deg.to_radians() + mount.pitch_factor * tilt + down + head_up,
    );
    // The roll turns the world's up about the bike's heading: a bike leaning right tips the camera's up to the right. A
    // camera on a lean follows the lean beyond its limit by the turn's roll.
    let with_the_lean = side(bike.roll) * mount.lean.turn_roll_deg.to_radians() * mount.lean.turn_share(lean_deg);
    let axis = direction(bike.yaw + into_bend, 0.0);
    let up = Quat::from_axis_angle(axis, mount.roll(bike.roll) + with_the_lean) * Vec3::Z;
    let mut lens = Lens::look_at(eye, eye + forward, mount.fov_y(aspect), mount.near, aspect);
    lens.view = glam::camera::rh::view::look_at_mat4(eye, eye + forward, up);
    lens
}

#[cfg(test)]
#[path = "bike_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "bike_fov_tests.rs"]
mod fov_tests;
