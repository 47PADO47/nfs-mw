//! The simple controller traffic uses: steer at the target, bang-bang pedals.
//! Spec: `docs/specs/ai-driver-control.md` (§4.1, §4.2).

use glam::Vec3;

use crate::throttle_pid::Pedals;

/// The signed angle from `forward` to the direction of `target`, in the xz plane: positive when the
/// target is on the right, within ±90°, together with the cosine between the two (negative when the
/// target is behind).
pub fn heading_to(position: Vec3, forward: Vec3, target: Vec3) -> (f32, f32) {
    let planar = |v: Vec3| Vec3::new(v.x, 0.0, v.z).try_normalize().unwrap_or(Vec3::ZERO);
    let (d, f) = (planar(target - position), planar(forward));
    let cross = f.z * d.x - f.x * d.z;
    (cross.clamp(-1.0, 1.0).asin(), f.dot(d))
}

/// Steering for the simple controller; also returns whether the target is behind the car.
pub fn simple_steering(
    position: Vec3,
    forward: Vec3,
    target: Vec3,
    max_steer: f32,
    reversing_gear: bool,
) -> (f32, bool) {
    let (angle, facing) = heading_to(position, forward, target);
    let mut steer = angle / max_steer.max(1e-3);
    let mut behind = false;
    match () {
        _ if reversing_gear => steer = -steer,
        _ if facing < -0.2 => {
            steer = if steer >= 0.0 { 1.0 } else { -1.0 };
            behind = true;
        }
        _ => {}
    }
    (steer.clamp(-1.0, 1.0), behind)
}

/// Pedals for the simple controller.
pub fn simple_pedals(
    speed: f32,
    want: f32,
    reversing_gear: bool,
    steering_behind: bool,
    reversing_speed: bool,
) -> Pedals {
    let none = Pedals::default();
    if !reversing_speed && steering_behind {
        return Pedals { gas: 1.0, brake: 0.0, handbrake: 1.0 };
    }
    if want < 0.5 {
        return Pedals { brake: 1.0, ..none };
    }
    if reversing_gear {
        return match speed > 1.0 {
            true => Pedals { brake: 1.0, ..none },
            false => Pedals { gas: 1.0, ..none },
        };
    }
    if speed < -1.0 {
        return Pedals { brake: 1.0, ..none };
    }
    if want < speed {
        // Brake hard only well above the wanted speed or when asked to nearly stop; otherwise coast.
        return match (want - speed).abs() > 2.5 || want < 5.0 {
            true => Pedals { brake: 1.0, ..none },
            false => none,
        };
    }
    Pedals { gas: 1.0, ..none }
}
