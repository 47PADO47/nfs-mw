//! Static wheel and brake transforms (front end: no spin, steering or suspension travel).
//! Spec: docs/specs/car-assembly.md §3–4. Car space is +x forward, +y left, +z up.

use glam::{Mat4, Vec3};

use super::ecar::{BikeMount, WheelSetup, is_front};

/// Degrees of camber per unit of `CamberFront` / `CamberRear`.
const CAMBER_DEGREES_PER_UNIT: f32 = 7.0;
/// How far camber lowers the wheel, metres per unit.
const CAMBER_PUSH_DOWN_PER_UNIT: f32 = 0.03;
/// Wheel size when the wheel solid is missing.
const FALLBACK_WIDTH: f32 = 0.225;
const FALLBACK_RADIUS: f32 = 0.32;
/// The front end sinks the tyres this far into the floor.
const FLOOR_SINK: f32 = 0.025;

/// The wheel solid's size, from its bounds: axle along y, rim face at y = 0, tyre towards +y.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelModel {
    pub width: f32,
    pub radius: f32,
}

impl WheelModel {
    pub fn from_bounds(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { width: max[1] - min[1], radius: (max[0] - min[0]) * 0.5 }
    }

    pub const FALLBACK: Self = Self { width: FALLBACK_WIDTH, radius: FALLBACK_RADIUS };
}

/// Where one corner's wheel and brake go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corner {
    pub wheel: Mat4,
    pub brake: Mat4,
    /// Left side (wheels 0 and 3): its brake is mirrored and uses the left caliper texture.
    pub left: bool,
    rig: Rig,
}

/// The pieces `wheel` and `brake` are made of, so a moving car can pose them.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rig {
    /// Axle centre and camber, car space.
    base: Mat4,
    /// Side flip and the move onto the wheel's axle pivot, before the size scale.
    wheel_pre: Mat4,
    wheel_scale: Mat4,
    /// The brake's mirror and its depth on the axle.
    brake_local: Mat4,
    centre: Vec3,
    /// The wheel mesh is turned round (not mirrored) on this side, so it spins the other way.
    spin_reversed: bool,
    /// How the wheel of a bike moves (`place_bike`); `None` for the wheel of a car.
    bike: Option<BikeRig>,
}

/// The axes a bike's wheel moves on. A bike's front wheel is steered about the steering axis, which leans back by the rake,
/// and travels along it as the fork compresses; the rear wheel travels straight up and does not steer.
#[derive(Debug, Clone, Copy, PartialEq)]
struct BikeRig {
    /// The unit axis the wheel turns about when it steers (positive turns left) and travels along (positive lifts).
    axis: Vec3,
    /// A point on that axis, car space.
    point: Vec3,
    /// Whether the travel moves it along the axis. The bars (corner 1, `place_bike`) steer with the fork and stay where
    /// they are along the axis.
    travels: bool,
}

/// How a corner moves while driving.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelPose {
    /// Steering angle in radians, positive turning left.
    pub steer: f32,
    /// Roll angle in radians, increasing as the car drives forward.
    pub spin: f32,
    /// Suspension travel, metres: positive lifts the wheel above its rest position.
    pub travel: f32,
}

impl Corner {
    /// The wheel's centre in car space at rest (camber included, no steering or travel).
    pub fn centre(&self) -> Vec3 {
        self.rig.centre
    }

    /// For a bike's wheel: the unit axis its steering turns about (positive turns left) and a point on it, car space. The
    /// fork and the handlebar turn about the same line, so a rider's hands can follow the bars. `None` for a car's wheel.
    pub fn steering_axis(&self) -> Option<(Vec3, Vec3)> {
        self.rig.bike.as_ref().map(|bike| (bike.axis, bike.point))
    }

    /// The wheel and brake transforms (car space) with the given steering, spin and travel.
    /// The brake steers and moves with the wheel but does not spin.
    pub fn posed(&self, pose: WheelPose) -> (Mat4, Mat4) {
        let rig = &self.rig;
        if let Some(bike) = &rig.bike {
            return rig.posed_bike(bike, pose);
        }
        let lift = Mat4::from_translation(Vec3::Z * pose.travel);
        let steer = Mat4::from_translation(rig.centre)
            * Mat4::from_rotation_z(pose.steer)
            * Mat4::from_translation(-rig.centre);
        let spin = Mat4::from_rotation_y(if rig.spin_reversed { -pose.spin } else { pose.spin });
        let wheel = lift * steer * rig.base * rig.wheel_pre * spin * rig.wheel_scale;
        let brake = lift * steer * rig.base * rig.brake_local;
        (wheel, brake)
    }
}

impl Rig {
    /// The wheel and brake of a bike's wheel: the travel and the steering are along and about the steering axis, the
    /// spin about the axle (the car's y axis through the hub).
    fn posed_bike(&self, bike: &BikeRig, pose: WheelPose) -> (Mat4, Mat4) {
        let lift = Mat4::from_translation(bike.axis * if bike.travels { pose.travel } else { 0.0 });
        let steer = Mat4::from_translation(bike.point)
            * Mat4::from_axis_angle(bike.axis, pose.steer)
            * Mat4::from_translation(-bike.point);
        let spin = Mat4::from_rotation_y(pose.spin);
        let wheel = lift * steer * self.base * self.wheel_pre * spin * self.wheel_scale;
        let brake = lift * steer * self.base * self.brake_local;
        (wheel, brake)
    }
}

/// The wheels of a bike (`docs/specs/vehicle-two-wheel-integration.md`): the front wheel in corner 0, the rear wheel in
/// corner 2, each a solid modelled with its axle along y and its hub at the origin, centred on the plane y = 0. Corner 1 is
/// the handlebar group: the front wheel's mount that steers with the fork but does not travel with it (its `wheel` is the
/// front wheel's, nothing is drawn with it, its `brake` places the bars solid). Corner 3 repeats the rear wheel so every
/// index holds a corner; nothing is placed on it. `front` and `rear` are the sizes of the two solids; the wheels are scaled
/// to the radius and width `setup` gives.
pub fn place_bike(setup: &WheelSetup, mount: &BikeMount, front: WheelModel, rear: WheelModel) -> [Corner; 4] {
    let (sin, cos) = mount.rake.sin_cos();
    let steering_axis = Vec3::new(-sin, 0.0, cos);
    let wheel = |index: usize, model: WheelModel| {
        let [x, _, z, radius] = setup.tire_offsets[index];
        let width = setup.skid_width[index];
        let ws = if width > 0.0 && model.width > 0.0 { width / model.width } else { 1.0 };
        let rs = if radius > 0.0 && model.radius > 0.0 { radius / model.radius } else { 1.0 };
        let centre = Vec3::new(x, 0.0, z);
        let corner = Mat4::from_translation(centre);
        let wheel_scale = Mat4::from_scale(Vec3::new(rs, ws, rs));
        // The steering axis meets the ground one trail ahead of the contact patch, which is one radius under the hub.
        let (axis, point) = match index {
            0 | 1 => (steering_axis, centre + Vec3::new(mount.trail, 0.0, -radius)),
            _ => (Vec3::Z, centre),
        };
        let rig = Rig {
            base: corner,
            wheel_pre: Mat4::IDENTITY,
            wheel_scale,
            brake_local: Mat4::IDENTITY,
            centre,
            spin_reversed: false,
            bike: Some(BikeRig { axis, point, travels: index != 1 }),
        };
        Corner { wheel: corner * wheel_scale, brake: corner, left: false, rig }
    };
    let (bars, front, rear) = (wheel(1, front), wheel(0, front), wheel(2, rear));
    [front, bars, rear, rear]
}

/// The four corners. `kit` is the body's kit number, `brake_marker_y` the wheel solid's
/// `FRONT_BRAKE`/`REAR_BRAKE` marker depth per end (0 when absent), and `camber` whether the
/// LOD shows camber (the game applies it at LOD A and B only).
pub fn place(setup: &WheelSetup, kit: usize, model: WheelModel, brake_marker_y: [f32; 2], camber: bool) -> [Corner; 4] {
    std::array::from_fn(|i| {
        let [x, y, z, radius] = setup.tire_offsets[i];
        let end = usize::from(!is_front(i));
        let left = i == 0 || i == 3;

        let skid = setup.skid_width[i] * setup.kit_scale(kit, i);
        let ws = if skid > 0.0 && model.width > 0.0 { skid / model.width } else { 1.0 };
        let rs = if radius > 0.0 && model.radius > 0.0 { radius / model.radius } else { 1.0 };
        let pivot = 0.5 * model.width * ws;
        let track = y.abs() + setup.kit_offset(kit, i);
        let z = z + setup.fe_compressions[end];
        let amount = if camber { setup.camber[end] } else { 0.0 };
        // Tops lean inwards: positive about +x on the left, negative on the right.
        let angle = (amount * CAMBER_DEGREES_PER_UNIT).to_radians() * if left { 1.0 } else { -1.0 };
        let axle_y = if left { track - pivot } else { -(track - pivot) };

        // Applied right to left: model space → centred on the pivot → side → camber → corner.
        let corner = Mat4::from_translation(Vec3::new(x, axle_y, z - amount * CAMBER_PUSH_DOWN_PER_UNIT))
            * Mat4::from_rotation_x(angle);
        let side = match (left, setup.spoke_count < 0) {
            (false, _) => Mat4::IDENTITY,
            (true, false) => Mat4::from_rotation_z(std::f32::consts::PI),
            (true, true) => Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0)),
        };
        let wheel_pre = side * Mat4::from_translation(Vec3::new(0.0, -pivot, 0.0));
        let wheel_scale = Mat4::from_scale(Vec3::new(rs, ws, rs));
        let mirror = if left { Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0)) } else { Mat4::IDENTITY };
        let brake_local = mirror * Mat4::from_translation(Vec3::new(0.0, brake_marker_y[end] * ws - pivot, 0.0));
        let rig = Rig {
            base: corner,
            wheel_pre,
            wheel_scale,
            brake_local,
            centre: Vec3::new(x, axle_y, z - amount * CAMBER_PUSH_DOWN_PER_UNIT),
            spin_reversed: left && setup.spoke_count >= 0,
            bike: None,
        };
        Corner { wheel: corner * wheel_pre * wheel_scale, brake: corner * brake_local, left, rig }
    })
}

/// Height of the car origin above the floor in the front end: the tyres rest on the floor and
/// sink in slightly.
pub fn floor_height(setup: &WheelSetup) -> f32 {
    let mean = |f: &dyn Fn(usize) -> f32| (0..4).map(f).sum::<f32>() / 4.0;
    let radius = mean(&|i| setup.tire_offsets[i][3]);
    let z = mean(&|i| setup.tire_offsets[i][2] + setup.fe_compressions[usize::from(!is_front(i))]);
    radius - z - FLOOR_SINK
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The BMW M3 GTR (docs/specs/car-assembly.md, worked example).
    fn m3() -> WheelSetup {
        WheelSetup {
            tire_offsets: [
                [1.615, 0.88, 0.0, 0.33],
                [1.615, -0.88, 0.0, 0.33],
                [-1.12, -0.89, 0.0, 0.33],
                [-1.12, 0.89, 0.0, 0.33],
            ],
            fe_compressions: [0.21, 0.21],
            skid_width: [0.235, 0.235, 0.255, 0.255],
            skid_width_kit_scale: vec![[1.0, 1.0]; 7],
            kit_wheel_offset: [vec![0.0; 6], vec![0.0; 6]],
            camber: [0.28, 0.20],
            spoke_count: 8,
            bike: None,
        }
    }

    const MODEL: WheelModel = WheelModel { width: 0.2469, radius: 0.3416 };

    /// |y| range of the drawn tyre; the solid's vertices span y = -0.0084..0.2385.
    fn y_span(m: Mat4) -> (f32, f32) {
        let a = m.transform_point3(Vec3::new(0.0, -0.0084, 0.0)).y;
        let b = m.transform_point3(Vec3::new(0.0, 0.2385, 0.0)).y;
        (a.abs().min(b.abs()), a.abs().max(b.abs()))
    }

    #[test]
    fn worked_example_without_camber() {
        let corners = place(&m3(), 0, MODEL, [0.046, 0.046], false);
        let close = |a: f32, b: f32| (a - b).abs() < 2e-3;
        for (i, (inner, outer)) in
            [(0.653, 0.888), (0.653, 0.888), (0.644, 0.899), (0.644, 0.899)].into_iter().enumerate()
        {
            let (a, b) = y_span(corners[i].wheel);
            assert!(close(a, inner) && close(b, outer), "wheel {i}: {a}..{b}");
            let centre = corners[i].wheel.transform_point3(Vec3::ZERO);
            assert!(close(centre.z, 0.21), "wheel {i} z {}", centre.z);
        }
        // Brake origins sit marker × width scale inboard of the rim face.
        let brake = |i: usize| corners[i].brake.transform_point3(Vec3::ZERO).y.abs();
        assert!(close(brake(0), 0.836) && close(brake(1), 0.836) && close(brake(2), 0.843), "{}", brake(2));
        // Rim faces point outwards on both sides.
        assert!(corners[0].wheel.transform_point3(Vec3::ZERO).y > 0.0);
        assert!(corners[1].wheel.transform_point3(Vec3::ZERO).y < 0.0);
    }

    #[test]
    fn camber_tilts_tops_inwards() {
        let corners = place(&m3(), 0, MODEL, [0.0; 2], true);
        for (i, c) in corners.iter().enumerate() {
            let top = c.wheel.transform_point3(Vec3::new(0.0, MODEL.width * 0.5, MODEL.radius));
            let bottom = c.wheel.transform_point3(Vec3::new(0.0, MODEL.width * 0.5, -MODEL.radius));
            assert!(top.y.abs() < bottom.y.abs(), "wheel {i}");
        }
    }

    #[test]
    fn rest_pose_is_the_static_placement() {
        let mirrored = WheelSetup { spoke_count: -5, ..m3() };
        for c in
            place(&m3(), 0, MODEL, [0.046, 0.046], true).into_iter().chain(place(&mirrored, 0, MODEL, [0.0; 2], true))
        {
            let (wheel, brake) = c.posed(WheelPose::default());
            assert!(wheel.abs_diff_eq(c.wheel, 1e-6) && brake.abs_diff_eq(c.brake, 1e-6));
        }
    }

    #[test]
    fn wheels_roll_forward_steer_and_lift() {
        let mirrored = WheelSetup { spoke_count: -5, ..m3() };
        let corners: Vec<Corner> = place(&m3(), 0, MODEL, [0.0; 2], false)
            .into_iter()
            .chain(place(&mirrored, 0, MODEL, [0.0; 2], false))
            .collect();
        for (i, c) in corners.iter().enumerate() {
            // A point on top of the tyre moves forward (+x) as the wheel rolls, on both sides.
            let top = Vec3::new(0.0, MODEL.width * 0.5, MODEL.radius);
            let before = c.posed(WheelPose::default()).0.transform_point3(top);
            let after = c.posed(WheelPose { spin: 0.1, ..WheelPose::default() }).0.transform_point3(top);
            assert!(after.x > before.x + 0.01, "wheel {i} rolls the wrong way");
            // The centre of the axle does not move with the spin.
            let centre = |p: WheelPose| c.posed(p).0.transform_point3(Vec3::new(0.0, MODEL.width * 0.5, 0.0));
            assert!(
                centre(WheelPose::default()).abs_diff_eq(centre(WheelPose { spin: 1.3, ..Default::default() }), 1e-4)
            );
            // Steering left swings the front of the tyre to the left (+y); travel lifts.
            // (turned-round left wheels have their mesh x axis pointing backwards)
            let front =
                Vec3::new(if c.rig.spin_reversed { -MODEL.radius } else { MODEL.radius }, MODEL.width * 0.5, 0.0);
            let steered = c.posed(WheelPose { steer: 0.2, ..Default::default() }).0.transform_point3(front);
            let straight = c.posed(WheelPose::default()).0.transform_point3(front);
            assert!(steered.y > straight.y + 0.01, "wheel {i} steers the wrong way");
            let up = c.posed(WheelPose { travel: 0.05, ..Default::default() });
            assert!((up.0.transform_point3(top).z - before.z - 0.05).abs() < 1e-5);
            assert!((up.1.transform_point3(Vec3::ZERO).z - c.brake.transform_point3(Vec3::ZERO).z - 0.05).abs() < 1e-5);
        }
    }

    #[test]
    fn floor() {
        assert!((floor_height(&m3()) - 0.095).abs() < 1e-4);
    }

    const BIKE_RAKE: f32 = 0.4538;
    const BIKE_TRAIL: f32 = 0.105;
    const BIKE_WHEEL: WheelModel = WheelModel { width: 0.12, radius: 0.324 };

    /// The hubs of a bike: the front 0.7355 m ahead of the model's origin, the rear 0.739 m behind it.
    fn bike_corners() -> [Corner; 4] {
        let hub = |x: f32| [x, 0.0, 0.324, 0.324];
        let setup = WheelSetup {
            tire_offsets: [hub(0.7355), hub(0.7355), hub(-0.739), hub(-0.739)],
            skid_width: [0.12, 0.12, 0.15, 0.15],
            bike: Some(BikeMount { rake: BIKE_RAKE, trail: BIKE_TRAIL }),
            ..m3()
        };
        place_bike(&setup, &setup.bike.unwrap(), BIKE_WHEEL, BIKE_WHEEL)
    }

    #[test]
    fn a_bike_wheel_is_centred_on_its_hub_and_scaled_to_its_width() {
        let corners = bike_corners();
        let front = corners[0].wheel.transform_point3(Vec3::ZERO);
        let rear = corners[2].wheel.transform_point3(Vec3::ZERO);
        assert!(front.abs_diff_eq(Vec3::new(0.7355, 0.0, 0.324), 1e-5), "{front}");
        assert!(rear.abs_diff_eq(Vec3::new(-0.739, 0.0, 0.324), 1e-5), "{rear}");
        // The rear tyre is wider than the front one: the mesh's width is stretched to it.
        let side = |c: &Corner| c.wheel.transform_vector3(Vec3::Y).length();
        assert!((side(&corners[0]) - 1.0).abs() < 1e-5 && (side(&corners[2]) - 1.25).abs() < 1e-5);
        // Corner 1 is the bars, which have the front wheel's mount; corner 3 repeats the rear wheel.
        assert_eq!(corners[1].wheel, corners[0].wheel);
        assert_eq!(corners[3], corners[2]);
    }

    #[test]
    fn the_rest_pose_of_a_bike_is_its_static_placement() {
        for corner in bike_corners() {
            let (wheel, brake) = corner.posed(WheelPose::default());
            assert!(wheel.abs_diff_eq(corner.wheel, 1e-6) && brake.abs_diff_eq(corner.brake, 1e-6));
        }
    }

    #[test]
    fn a_bike_wheel_rolls_forward_about_its_hub() {
        let corners = bike_corners();
        for corner in [corners[0], corners[2]] {
            let top = Vec3::new(0.0, 0.0, 0.324);
            let before = corner.posed(WheelPose::default()).0.transform_point3(top);
            let after = corner.posed(WheelPose { spin: 0.1, ..WheelPose::default() }).0.transform_point3(top);
            assert!(after.x > before.x + 0.01, "the wheel rolls the wrong way");
            let hub =
                |spin: f32| corner.posed(WheelPose { spin, ..WheelPose::default() }).0.transform_point3(Vec3::ZERO);
            assert!(hub(0.0).abs_diff_eq(hub(2.0), 1e-5), "the hub does not move with the spin");
        }
    }

    #[test]
    fn the_front_wheel_steers_about_the_raked_axis_and_travels_along_it() {
        let front = bike_corners()[0];
        let hub = |pose: WheelPose| front.posed(pose).0.transform_point3(Vec3::ZERO);
        let straight = hub(WheelPose::default());
        // Steering left swings the front of the tyre to the left (+y).
        let nose = Vec3::new(0.324, 0.0, 0.0);
        let turned = front.posed(WheelPose { steer: 0.2, ..WheelPose::default() }).0.transform_point3(nose);
        assert!(turned.y > straight.y + 0.03, "{turned}");
        // The point where the axis meets the ground stays where it is whatever the steering: the hub swings round it.
        let ground = Vec3::new(0.7355 + BIKE_TRAIL, 0.0, 0.0);
        let steered = front.posed(WheelPose { steer: 0.5, ..WheelPose::default() });
        let foot = steered.1.transform_point3(Vec3::new(BIKE_TRAIL, 0.0, -0.324));
        assert!((foot - ground).length() < 1e-4, "{foot}");
        // The fork compresses along the axis: up and back, by the rake.
        let up = hub(WheelPose { travel: 0.05, ..WheelPose::default() }) - straight;
        let axis = Vec3::new(-BIKE_RAKE.sin(), 0.0, BIKE_RAKE.cos());
        assert!((up - axis * 0.05).length() < 1e-5, "{up}");
    }

    #[test]
    fn the_rear_wheel_travels_straight_up() {
        let rear = bike_corners()[2];
        let hub = |travel: f32| rear.posed(WheelPose { travel, ..WheelPose::default() }).0.transform_point3(Vec3::ZERO);
        assert!((hub(0.04) - hub(0.0) - Vec3::new(0.0, 0.0, 0.04)).length() < 1e-5);
    }

    #[test]
    fn a_bikes_front_wheel_reports_its_steering_axis_and_a_cars_wheel_has_none() {
        let (axis, point) = bike_corners()[0].steering_axis().unwrap();
        assert!(axis.abs_diff_eq(Vec3::new(-BIKE_RAKE.sin(), 0.0, BIKE_RAKE.cos()), 1e-6));
        assert!(point.abs_diff_eq(Vec3::new(0.7355 + BIKE_TRAIL, 0.0, 0.0), 1e-5), "{point}");
        assert!(place(&m3(), 0, MODEL, [0.0; 2], false)[0].steering_axis().is_none());
    }

    /// The handlebar group (`assemble.rs`, the bars solid, corner 1) is the fork without the travel: it steers about the
    /// same axis and stays where it is along it.
    #[test]
    fn the_bars_steer_with_the_fork_but_do_not_travel_with_it() {
        let corners = bike_corners();
        let (fork, bars) = (corners[0], corners[1]);
        let rest = bars.posed(WheelPose::default()).1;
        assert!(rest.transform_point3(Vec3::ZERO).abs_diff_eq(Vec3::new(0.7355, 0.0, 0.324), 1e-5));
        let travelled = bars.posed(WheelPose { travel: 0.08, ..WheelPose::default() }).1;
        assert!(travelled.abs_diff_eq(rest, 1e-6), "the bars do not slide");
        let slid = fork.posed(WheelPose { travel: 0.08, ..WheelPose::default() }).1;
        assert!(!slid.abs_diff_eq(rest, 1e-3), "the fork does");
        for steer in [0.0, 0.3, -0.4] {
            let pose = WheelPose { steer, ..WheelPose::default() };
            assert!(bars.posed(pose).1.abs_diff_eq(fork.posed(pose).1, 1e-6), "steering turns both alike");
        }
        assert!(bars.steering_axis() == fork.steering_axis());
    }

    /// The front end of a bike (`assemble.rs`, the fork solid) is placed with the front corner's `brake` matrix: the solid
    /// has the front hub on its origin, and the matrix steers it about the raked axis and moves it along the axis with the
    /// wheel, but neither spins nor scales it.
    #[test]
    fn the_fork_is_posed_like_the_front_wheel_without_its_spin_and_scale() {
        let front = bike_corners()[0];
        let hub = Vec3::new(0.7355, 0.0, 0.324);
        let rest = front.posed(WheelPose::default()).1;
        assert!(rest.transform_point3(Vec3::ZERO).abs_diff_eq(hub, 1e-5), "the fork's origin is the hub at rest");
        assert!(front.brake.abs_diff_eq(rest, 1e-6));
        // The roll does not turn it.
        assert!(front.posed(WheelPose { spin: 2.0, ..WheelPose::default() }).1.abs_diff_eq(rest, 1e-6));
        // Its origin goes where the wheel's hub goes, whatever the steering and the travel.
        let pose = WheelPose { steer: 0.3, spin: 1.0, travel: 0.04 };
        let (wheel, fork) = front.posed(pose);
        assert!(wheel.transform_point3(Vec3::ZERO).abs_diff_eq(fork.transform_point3(Vec3::ZERO), 1e-5));
        // Not scaled: the wheel is stretched to the tyre's width, the fork is not.
        assert!((fork.transform_vector3(Vec3::Y).length() - 1.0).abs() < 1e-5);
        assert!((fork.transform_vector3(Vec3::X).length() - 1.0).abs() < 1e-5);
        // A point of the steering axis (the axis meets the ground one trail ahead of the contact patch, which is one
        // radius under the hub) stays on the axis when the fork steers: it does not move.
        let axis = Vec3::new(-BIKE_RAKE.sin(), 0.0, BIKE_RAKE.cos());
        let on_axis = Vec3::new(BIKE_TRAIL, 0.0, -0.324) + axis * 0.6;
        let steered = front.posed(WheelPose { steer: 0.5, ..WheelPose::default() }).1.transform_point3(on_axis);
        assert!(steered.abs_diff_eq(hub + on_axis, 1e-4), "{steered}");
        // Steering left swings the fork's front to the left.
        let nose = front.posed(WheelPose { steer: 0.2, ..WheelPose::default() }).1.transform_point3(Vec3::X);
        assert!(nose.y > rest.transform_point3(Vec3::X).y + 0.03);
    }
}
