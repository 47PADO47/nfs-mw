//! One wheel on the ground: its pose, the compression that puts its tyre on the road, and what it reports.

use glam::Vec3;

use super::geometry::contact_point;
use super::spec::WheelSpec;
use crate::ground::{Ground, SurfaceGrip};

/// How far past the rest travel the compression may run before the bump stop takes the rest (m).
pub const OVER_TRAVEL: f32 = 0.03;
/// A tyre this close to the ground (m) counts as touching it.
const TOUCH: f32 = 0.004;
/// How far above the contact patch the ground ray starts, and how far it reaches (m).
const RAY_LIFT: f32 = 0.6;
const RAY_REACH: f32 = 2.0;

/// A wheel's pose in the world.
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub hub: Vec3,
    /// The unit axle, pointing right.
    pub axle: Vec3,
    /// The unit direction the wheel rolls along.
    pub heading: Vec3,
    /// The unit suspension axis, pointing the way compression moves the hub.
    pub axis: Vec3,
}

/// Where a tyre touches the ground.
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub point: Vec3,
    pub normal: Vec3,
    pub surface: SurfaceGrip,
    pub compression: f32,
    /// How much of a force along the suspension axis acts along the ground normal.
    pub axis_cosine: f32,
    pub axle: Vec3,
    pub heading: Vec3,
}

/// The ground's own axes at a contact: the way the wheel rolls and the way to its right.
#[derive(Clone, Copy, Debug)]
pub struct GroundFrame {
    pub forward: Vec3,
    pub right: Vec3,
}

pub fn ground_frame(normal: Vec3, heading: Vec3) -> GroundFrame {
    let flat = heading - normal * heading.dot(normal);
    let forward = flat.try_normalize().unwrap_or(heading);
    GroundFrame { forward, right: normal.cross(forward) }
}

/// What a wheel reports after a step.
#[derive(Clone, Debug, Default)]
pub struct WheelState {
    /// The compression beyond rest (m).
    pub compression: f32,
    /// The spin about the axle (rad/s), positive rolling forwards.
    pub spin: f32,
    pub on_ground: bool,
    /// The load on the tyre (N).
    pub load: f32,
    pub contact: Vec3,
    pub normal: Vec3,
    /// The whole force of the ground on the wheel (N).
    pub force: Vec3,
    /// The tyre's slip angle (rad) and slip ratio (1 = 100 percent).
    pub slip_angle: f32,
    pub slip_ratio: f32,
    pub camber: f32,
    pub surface_tag: u32,
}

impl WheelState {
    /// The wheel has left the ground: the suspension hangs at `compression` and nothing pushes on the tyre.
    pub fn release(&mut self, compression: f32) {
        self.compression = compression;
        self.on_ground = false;
        self.load = 0.0;
        self.force = Vec3::ZERO;
        self.slip_angle = 0.0;
        self.slip_ratio = 0.0;
    }
}

struct Sample {
    point: Vec3,
    normal: Vec3,
    surface: SurfaceGrip,
    /// The height of the tyre's lowest point over the ground, along the ground normal (m).
    gap: f32,
}

/// The lowest point of the tyre of a wheel at `pose` and its height over the ground below it.
fn probe(pose: &Pose, wheel: &WheelSpec, ground: &dyn Ground) -> Option<Sample> {
    let mut normal = Vec3::Y;
    let mut sample = None;
    for _ in 0..2 {
        let point = contact_point(pose.hub, pose.axle, normal, wheel);
        let origin = point + Vec3::Y * RAY_LIFT;
        let hit = ground.hit(origin, Vec3::NEG_Y, RAY_REACH)?;
        normal = hit.normal;
        let ground_point = origin + Vec3::NEG_Y * hit.distance;
        sample = Some(Sample { point, normal, surface: hit.surface, gap: (point - ground_point).dot(normal) });
    }
    sample
}

/// Find the suspension compression, within `limits`, that puts the tyre on the ground, starting from `start`. `pose_of`
/// gives the wheel's pose for a compression. `None` when there is no ground in reach or the wheel hangs clear of it at
/// full extension.
pub fn settle(
    pose_of: impl Fn(f32) -> Pose,
    wheel: &WheelSpec,
    ground: &dyn Ground,
    start: f32,
    limits: (f32, f32),
) -> Option<Contact> {
    let mut compression = start.clamp(limits.0, limits.1);
    for _ in 0..6 {
        let pose = pose_of(compression);
        let sample = probe(&pose, wheel, ground)?;
        let slope = pose.axis.dot(sample.normal).max(0.2);
        let next = (compression - sample.gap / slope).clamp(limits.0, limits.1);
        let settled = (next - compression).abs() < 1e-5;
        compression = next;
        if settled {
            break;
        }
    }
    let pose = pose_of(compression);
    let sample = probe(&pose, wheel, ground)?;
    if sample.gap > TOUCH {
        return None;
    }
    Some(Contact {
        point: sample.point,
        normal: sample.normal,
        surface: sample.surface,
        compression,
        axis_cosine: pose.axis.dot(sample.normal),
        axle: pose.axle,
        heading: pose.heading,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;
    use crate::ground::FlatGround;

    fn pose_at(height: f32) -> impl Fn(f32) -> Pose {
        move |compression| Pose {
            hub: Vec3::new(0.0, height + compression, 0.0),
            axle: Vec3::X,
            heading: Vec3::Z,
            axis: Vec3::Y,
        }
    }

    #[test]
    fn a_hub_over_the_ground_compresses_until_the_tyre_touches() {
        let spec = BikeSpec::example().front;
        let ground = FlatGround::new(0.0);
        let contact = settle(pose_at(spec.radius - 0.05), &spec, &ground, 0.0, (-0.1, 0.3)).unwrap();
        assert!((contact.compression - 0.05).abs() < 1e-4, "{}", contact.compression);
        assert!(contact.point.y.abs() < 1e-4);
    }

    #[test]
    fn a_hub_too_high_to_touch_is_airborne() {
        let spec = BikeSpec::example().front;
        let ground = FlatGround::new(0.0);
        assert!(settle(pose_at(spec.radius + 0.2), &spec, &ground, 0.0, (-0.1, 0.3)).is_none());
    }

    #[test]
    fn no_ground_is_no_contact() {
        let spec = BikeSpec::example().front;
        assert!(settle(pose_at(spec.radius), &spec, &crate::ground::NoGround, 0.0, (-0.1, 0.3)).is_none());
    }

    #[test]
    fn the_ground_frame_is_right_handed() {
        let frame = ground_frame(Vec3::Y, Vec3::Z);
        assert!((frame.right - Vec3::X).length() < 1e-6);
        let tilted = ground_frame(Vec3::new(0.0, 0.8, -0.6).normalize(), Vec3::Z);
        assert!(tilted.forward.dot(Vec3::new(0.0, 0.8, -0.6).normalize()).abs() < 1e-5);
    }

    #[test]
    fn releasing_a_wheel_clears_its_forces() {
        let mut state = WheelState { on_ground: true, load: 1000.0, force: Vec3::Y, ..Default::default() };
        state.release(-0.05);
        assert!(!state.on_ground && state.load == 0.0 && state.force == Vec3::ZERO);
        assert_eq!(state.compression, -0.05);
    }
}
