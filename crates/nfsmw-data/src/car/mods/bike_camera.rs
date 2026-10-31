//! Where the game's cameras sit on a bike a mod adds (`[bike.camera]` of its manifest, docs/modding/bikes.md and
//! docs/specs/ride6-bike-cameras.md): the numbers of the two chase views, of the camera ahead of the bike and of the camera inside
//! the rider's helmet, by car folder. The manifest's table is optional and partial; what it leaves out is derived from the bike's
//! size by [`BikeCamera::derived`], so every bike has a complete set.
//!
//! Space is the model's: x forward, y left, z up, metres, the origin on the ground.

use std::sync::{Arc, PoisonError, RwLock};

/// A third-person view behind the bike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chase {
    /// Behind the model's origin, m.
    pub distance: f32,
    pub height: f32,
    /// The point looked at: ahead of the origin and above the ground, m.
    pub target_ahead: f32,
    pub target_height: f32,
    /// Horizontal field of view, degrees.
    pub fov: f32,
}

/// The camera ahead of the bike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Front {
    pub position: [f32; 3],
    /// Looks back at the bike (else ahead down the road).
    pub look_back: bool,
    /// Degrees from the bike's axis, positive up.
    pub pitch_deg: f32,
    pub fov: f32,
    pub near: f32,
    /// The share of the bike's pitch and roll the camera takes, 0 to 1.
    pub pitch_factor: f32,
    pub roll_factor: f32,
}

/// The camera inside the helmet.
#[derive(Debug, Clone, PartialEq)]
pub struct Helmet {
    /// The rider's bones whose middle is the eye point, by name; `head_bone` and `head_eye_offset` when none is found.
    pub eye_bones: Vec<String>,
    pub head_bone: String,
    pub head_eye_offset: [f32; 3],
    /// The rider's meshes that are not drawn from inside the helmet.
    pub hide_meshes: Vec<String>,
    /// Where the eyes are without a rider, m.
    pub fixed_position: [f32; 3],
    pub pitch_deg: f32,
    pub fov: f32,
    pub near: f32,
    pub pitch_factor: f32,
    pub roll_factor: f32,
    /// The roll of the bike (degrees) followed beyond `roll_factor`, up to this much.
    pub follow_roll_max_deg: f32,
    /// How the camera moves and turns as the bike leans into a bend.
    pub lean: Lean,
}

/// What the helmet camera does with the bike's lean, on top of the roll it follows: the camera is not bolted to the leaning bike
/// (its position keeps the bike's heading and pitch and only the followed share of its roll), it is moved by a shift that grows
/// linearly with the lean between two angles, and turned by a second that grows between two others. RIDE 6 has them as the two
/// `Lean` slots of the helmet camera's modifier behaviour (`docs/specs/ride6-bike-cameras.md`, section 6). The sizes are for a
/// lean to one side: the sideways ones are toward the inside of the bend, so they flip with the lean.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lean {
    /// The shift grows from nothing at `shift_from_deg` of lean to all of it at `shift_to_deg`.
    pub shift_from_deg: f32,
    pub shift_to_deg: f32,
    /// Ahead, toward the inside of the bend and up, m (the data's 28 cm ahead, 16 inward and 35 down at 70 degrees on most bikes).
    pub shift_m: [f32; 3],
    /// The look is tilted this much at the full shift, degrees, positive up.
    pub shift_pitch_deg: f32,
    /// The turn grows from nothing at `turn_from_deg` of lean to all of it at `turn_to_deg`.
    pub turn_from_deg: f32,
    pub turn_to_deg: f32,
    /// How far the look turns toward the inside of the bend and how far the camera rolls with the lean at the full turn, degrees.
    pub turn_yaw_deg: f32,
    pub turn_roll_deg: f32,
}

impl Lean {
    /// A camera that does nothing with the lean (a mount that is bolted to the bike has no use for it).
    pub const NONE: Self = Self {
        shift_from_deg: 0.0,
        shift_to_deg: 0.0,
        shift_m: [0.0; 3],
        shift_pitch_deg: 0.0,
        turn_from_deg: 0.0,
        turn_to_deg: 0.0,
        turn_yaw_deg: 0.0,
        turn_roll_deg: 0.0,
    };

    /// What a bike whose manifest has no `[bike.camera.helmet.lean]` gets: the numbers of RIDE 6's root camera container, which
    /// most of its bikes repeat to within a few centimetres (the Panigale V4 R has 30, 15, -35; the KTM 450 SMR 10, 15, -10).
    pub const ROOT: Self = Self {
        shift_from_deg: 25.0,
        shift_to_deg: 70.0,
        shift_m: [0.28, 0.16, -0.35],
        shift_pitch_deg: -5.0,
        turn_from_deg: 30.0,
        turn_to_deg: 70.0,
        turn_yaw_deg: 20.0,
        turn_roll_deg: 12.0,
    };

    /// How much of the shift a lean of `lean_deg` (either side) has made, 0 to 1.
    pub fn shift_share(&self, lean_deg: f32) -> f32 {
        ramp(lean_deg.abs(), self.shift_from_deg, self.shift_to_deg)
    }

    /// How much of the turn a lean of `lean_deg` (either side) has made, 0 to 1.
    pub fn turn_share(&self, lean_deg: f32) -> f32 {
        ramp(lean_deg.abs(), self.turn_from_deg, self.turn_to_deg)
    }
}

/// 0 below `from`, 1 above `to`, linear between (a step at `to` when the two meet or cross).
fn ramp(value: f32, from: f32, to: f32) -> f32 {
    if to > from {
        return ((value - from) / (to - from)).clamp(0.0, 1.0);
    }
    f32::from(to > 0.0 && value >= to)
}

/// The cameras of one bike.
#[derive(Debug, Clone, PartialEq)]
pub struct BikeCamera {
    pub chase_near: Chase,
    pub chase_far: Chase,
    pub front: Front,
    pub helmet: Helmet,
}

/// The sizes of a bike, in wheelbases, the derived cameras are made from. Modelling choices (a manifest with no `[bike.camera]`
/// has no measured camera): the numbers are the proportions of a chase camera a rider would pick, a car-sized camera shrunk to a
/// bike by its length.
mod proportions {
    /// The close and the far chase camera, behind the origin and above the ground.
    pub const NEAR_DISTANCE: f32 = 2.3;
    pub const FAR_DISTANCE: f32 = 2.8;
    pub const NEAR_HEIGHT: f32 = 0.7;
    pub const FAR_HEIGHT: f32 = 1.0;
    /// The look point of the chase cameras.
    pub const TARGET_AHEAD: f32 = 0.2;
    pub const TARGET_HEIGHT: f32 = 0.5;
    /// The camera ahead of the bike: beyond the front hub, and its height.
    pub const FRONT_AHEAD_OF_HUB: f32 = 1.1;
    pub const FRONT_HEIGHT: f32 = 0.75;
    /// The eye of a rider: how far ahead of the origin and how high, with the head over the tank.
    pub const EYE_AHEAD: f32 = 0.05;
    pub const EYE_HEIGHT: f32 = 0.85;
}

/// The field of view of a camera with no number, degrees, and the near plane, m.
const DEFAULT_FOV: f32 = 85.0;
const DEFAULT_NEAR: f32 = 0.1;

impl BikeCamera {
    /// The cameras of a bike `wheelbase` metres long whose front hub is `front_hub_x` ahead of the model's origin, for a bike
    /// whose manifest gives no `[bike.camera]`.
    pub fn derived(wheelbase: f32, front_hub_x: f32) -> Self {
        use proportions as p;
        let chase = |distance: f32, height: f32| Chase {
            distance: distance * wheelbase,
            height: height * wheelbase,
            target_ahead: p::TARGET_AHEAD * wheelbase,
            target_height: p::TARGET_HEIGHT * wheelbase,
            fov: DEFAULT_FOV,
        };
        Self {
            chase_near: chase(p::NEAR_DISTANCE, p::NEAR_HEIGHT),
            chase_far: chase(p::FAR_DISTANCE, p::FAR_HEIGHT),
            front: Front {
                position: [front_hub_x + p::FRONT_AHEAD_OF_HUB * wheelbase, 0.0, p::FRONT_HEIGHT * wheelbase],
                look_back: true,
                pitch_deg: -10.0,
                fov: DEFAULT_FOV,
                near: DEFAULT_NEAR,
                pitch_factor: 0.0,
                roll_factor: 0.0,
            },
            helmet: Helmet {
                eye_bones: Vec::new(),
                head_bone: String::new(),
                head_eye_offset: [0.0; 3],
                hide_meshes: Vec::new(),
                fixed_position: [p::EYE_AHEAD * wheelbase, 0.0, p::EYE_HEIGHT * wheelbase],
                pitch_deg: -10.0,
                fov: DEFAULT_FOV,
                near: DEFAULT_NEAR,
                pitch_factor: 0.0,
                roll_factor: 0.0,
                follow_roll_max_deg: 0.0,
                lean: Lean::ROOT,
            },
        }
    }
}

/// The cameras of this run's bikes, by folder. Each publication leaks the new list, as the other per-car tables do.
static PUBLISHED: RwLock<&'static [(String, Arc<BikeCamera>)]> = RwLock::new(&[]);

/// Register the cameras of the bike in `folder`. A folder already registered keeps its first.
pub fn publish(folder: &str, cameras: BikeCamera) {
    let mut published = PUBLISHED.write().unwrap_or_else(PoisonError::into_inner);
    if published.iter().any(|(known, _)| known.eq_ignore_ascii_case(folder)) {
        return;
    }
    let mut all: Vec<(String, Arc<BikeCamera>)> = published.to_vec();
    all.push((folder.to_owned(), Arc::new(cameras)));
    *published = Box::leak(all.into_boxed_slice());
}

/// The cameras of the bike in `folder`; `None` for a stock car and for a mod car that is not a bike.
pub fn of(folder: &str) -> Option<Arc<BikeCamera>> {
    let published = *PUBLISHED.read().unwrap_or_else(PoisonError::into_inner);
    published.iter().find(|(known, _)| known.eq_ignore_ascii_case(folder)).map(|(_, cameras)| Arc::clone(cameras))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_cameras_scale_with_the_wheelbase_and_the_far_view_is_behind_and_above_the_near_one() {
        let small = BikeCamera::derived(1.2, 0.6);
        let long = BikeCamera::derived(1.8, 0.9);
        for (a, b) in [(small.chase_near, long.chase_near), (small.chase_far, long.chase_far)] {
            assert!((b.distance / a.distance - 1.5).abs() < 1e-5 && (b.height / a.height - 1.5).abs() < 1e-5);
        }
        assert!(
            small.chase_far.distance > small.chase_near.distance && small.chase_far.height > small.chase_near.height
        );
        assert!(small.front.position[0] > 0.6, "the front camera is ahead of the front hub");
        assert!(small.front.look_back);
        assert!(small.helmet.fixed_position[2] > small.chase_near.target_height, "the eyes are above the look point");
    }

    #[test]
    fn the_lean_shares_grow_linearly_between_their_angles_and_flip_with_the_side() {
        let lean = Lean::ROOT;
        assert_eq!(lean.shift_share(0.0), 0.0);
        assert_eq!(lean.shift_share(25.0), 0.0);
        assert!((lean.shift_share(47.5) - 0.5).abs() < 1e-6);
        assert_eq!(lean.shift_share(-70.0), 1.0, "the other side");
        assert_eq!(lean.shift_share(90.0), 1.0, "no more than all of it");
        assert!((lean.turn_share(50.0) - 0.5).abs() < 1e-6);
        assert_eq!(Lean::NONE.shift_share(60.0), 0.0);
        assert_eq!(Lean::NONE.turn_share(60.0), 0.0);
    }

    #[test]
    fn a_published_bike_is_found_by_folder_without_regard_to_case_and_the_first_publication_wins() {
        publish("TESTBIKE_CAM", BikeCamera::derived(1.4, 0.7));
        publish("testbike_cam", BikeCamera::derived(2.0, 1.0));
        let found = of("testbike_Cam").expect("published");
        assert!((found.chase_near.distance - 2.3 * 1.4).abs() < 1e-5);
        assert!(of("NOT_A_BIKE").is_none());
    }
}
