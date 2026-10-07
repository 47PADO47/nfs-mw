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
        let wheel = corner
            * side
            * Mat4::from_translation(Vec3::new(0.0, -pivot, 0.0))
            * Mat4::from_scale(Vec3::new(rs, ws, rs));
        let mirror = if left { Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0)) } else { Mat4::IDENTITY };
        let brake = corner * mirror * Mat4::from_translation(Vec3::new(0.0, brake_marker_y[end] * ws - pivot, 0.0));
        Corner { wheel, brake, left }
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
    fn floor() {
        assert!((floor_height(&m3()) - 0.095).abs() < 1e-4);
    }
}
