//! Original articulation angle frames and limit-plane response.
//! Spec: `docs/specs/trailer-constraints.md`.

use glam::{Quat, Vec3};

use super::{BallJoint, BodyHit, BodyState, PlaneContact, RigidBody, react_moments};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimitMode {
    Prismatic,
    Conical,
}

#[derive(Clone, Copy, Debug)]
pub struct AngularLimit {
    pub frame_a: Quat,
    pub vector_b: Vec3,
    pub positive: f32,
    pub negative: f32,
    pub mode: LimitMode,
}

impl AngularLimit {
    /// Return world constraint frame and exceeded angle, in radians.
    pub fn exceeded(&self, a: &RigidBody, b: &RigidBody) -> Option<(Quat, f32)> {
        let vector = b.rotation() * self.vector_b;
        let mut frame = a.orientation * self.frame_a;
        if self.mode == LimitMode::Conical {
            let local = frame.conjugate() * vector;
            let x = Quat::from_rotation_x(-local.y.atan2(local.z));
            let z = Quat::from_rotation_z(local.y.atan2(local.x));
            frame *= z * x;
        }
        let local = frame.conjugate() * vector;
        let angle = local.x.atan2(local.z);
        if angle > self.positive {
            return Some((frame, self.positive - angle));
        }
        if angle < -self.negative {
            return Some((frame, -self.negative - angle));
        }
        None
    }

    pub fn solve(&self, joint: &BallJoint, a: &mut RigidBody, b: &mut RigidBody) {
        let Some((frame, delta)) = self.exceeded(a, b) else { return };
        let fulcrum = joint.fulcrum(a, b);
        let share = match (a.state == BodyState::Frozen, b.state == BodyState::Frozen) {
            (true, _) => 1.0,
            (_, true) => 0.0,
            _ => a.mass() / (a.mass() + b.mass()),
        };
        turn(a, joint.anchor_a, fulcrum, frame, -(1.0 - share) * delta);
        turn(b, joint.anchor_b, fulcrum, frame, share * delta);
        let (normal, point) = self.plane(frame, fulcrum, delta < 0.0);
        let mut ma = a.contact_moment(&BodyHit { immobile: a.state == BodyState::Frozen, ..Default::default() });
        let mut mb = b.contact_moment(&BodyHit { immobile: b.state == BodyState::Frozen, ..Default::default() });
        let plane = PlaneContact { point, normal, friction_static: 0.0, friction_kinetic: 0.0 };
        if react_moments(&mut ma, &mut mb, &plane, 32).is_some() {
            a.apply_contact_moment(&ma);
            b.apply_contact_moment(&mb);
        }
    }

    fn plane(&self, frame: Quat, fulcrum: Vec3, positive: bool) -> (Vec3, Vec3) {
        let angle = match positive {
            true => self.positive,
            false => self.negative,
        };
        let sign = match positive {
            true => 1.0,
            false => -1.0,
        };
        let (sin, cos) = angle.sin_cos();
        let normal = frame * Vec3::new(-sign * cos, 0.0, sin);
        let point = fulcrum + frame * (Vec3::new(sign * sin, 0.0, cos) * self.vector_b.length());
        (normal, point)
    }
}

fn turn(body: &mut RigidBody, anchor: Vec3, fulcrum: Vec3, frame: Quat, delta: f32) {
    if body.state == BodyState::Frozen {
        return;
    }
    let orientation = frame * Quat::from_rotation_y(delta) * frame.conjugate() * body.orientation;
    body.set_orientation(orientation);
    body.position = fulcrum - body.rotation() * anchor;
}

#[cfg(test)]
mod tests;
