//! Drag and downforce. Spec section 5 of `docs/specs/vehicle-steering-assists-aero.md`.

use glam::{Mat3, Vec3};

/// Aerodynamic parameters (from the attribute class `chassis`).
#[derive(Clone, Copy, Debug, Default)]
pub struct AeroSpec {
    /// Speed-proportional drag coefficient: the drag force is `coefficient * speed * velocity`, so it grows
    /// with the square of the speed. 0 disables drag.
    pub drag_coefficient: f32,
    /// Downforce per m/s, in kN-ish data units: the force is `speed * 2 * coefficient * 1000` newtons.
    /// 0 disables downforce.
    pub aero_coefficient: f32,
    /// Where the downforce acts, in percent of the wheelbase from the rear axle toward the front.
    pub aero_cg: f32,
}

/// Inputs of the aerodynamic forces.
#[derive(Clone, Copy, Debug)]
pub struct AeroInput {
    pub linear_velocity: Vec3,
    /// Body orientation (columns are the world right, up and forward axes).
    pub rotation: Mat3,
    /// Gas pedal 0..1.
    pub gas: f32,
    /// `0.25 * wheels on the ground` from the previous step.
    pub ground_effect: f32,
    /// Any wheel touches the ground now.
    pub any_wheel_on_ground: bool,
    /// Aerodynamics tuning slider in [-1, 1].
    pub tuning: f32,
}

/// The forces of one step.
#[derive(Clone, Copy, Debug, Default)]
pub struct AeroForces {
    /// Drag (N, world frame), applied at the centre of gravity lowered by `drag_drop` metres.
    pub drag: Vec3,
    /// How far below the centre of gravity (along the body up axis) the drag acts.
    pub drag_drop: f32,
    /// Downforce (N, world frame, along minus the body up axis).
    pub downforce: Vec3,
    /// Body-frame z where the downforce acts, or `None` for the centre of gravity.
    pub downforce_z: Option<f32>,
}

/// Computes drag and downforce. `front_z` and `rear_z` are the body-frame z of the front and rear axles.
pub fn forces(spec: &AeroSpec, i: &AeroInput, front_z: f32, rear_z: f32) -> AeroForces {
    let mut out = AeroForces::default();
    let speed = i.linear_velocity.length();
    let tuning = i.tuning.clamp(-1.0, 1.0);

    if spec.drag_coefficient != 0.0 {
        let mut drag = speed * spec.drag_coefficient;
        drag += drag * (1.0 - i.gas.clamp(0.0, 1.0));
        drag += drag * 0.25 * tuning;
        out.drag = -i.linear_velocity * drag;
        if i.ground_effect >= 0.5 {
            out.drag_drop = 0.1 * (1.0 - i.gas.clamp(0.0, 1.0));
        }
    }

    if spec.aero_coefficient != 0.0 {
        let up = i.rotation.y_axis;
        let forward = i.rotation.z_axis;
        let upness = if i.ground_effect >= 0.5 { 1.0 } else { up.y.max(0.0) };
        let move_dir = if speed > 1e-4 { i.linear_velocity / speed } else { forward };
        let forwardness = move_dir.dot(forward).max(0.0).sqrt().max(0.4);
        let mut downforce = upness * forwardness * speed * 2.0 * spec.aero_coefficient * 1000.0;
        if !i.any_wheel_on_ground {
            downforce *= 0.8;
        }
        downforce *= 1.0 + 0.25 * tuning;
        out.downforce = -up * downforce;
        if i.ground_effect != 0.0 {
            out.downforce_z = Some((front_z - rear_z) * spec.aero_cg * 0.01 + rear_z);
        }
    }
    out
}

#[cfg(test)]
mod tests;
