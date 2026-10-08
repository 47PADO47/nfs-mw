//! Static wheel and brake transforms (front end: no spin, steering or suspension travel).
//! Spec: docs/specs/car-assembly.md §3–4. Car space is +x forward, +y left, +z up.

use glam::{Mat4, Vec3};

use super::ecar::{WheelSetup, is_front};

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

    /// The wheel and brake transforms (car space) with the given steering, spin and travel.
    /// The brake steers and moves with the wheel but does not spin.
    pub fn posed(&self, pose: WheelPose) -> (Mat4, Mat4) {
        let rig = &self.rig;
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
}
