//! Articulation: mass-weighted fulcrum correction followed by joint moment response.
//! Spec: `docs/specs/trailer-constraints.md`.

use glam::{Mat3, Quat, Vec3};

use super::{BodyHit, BodyState, RigidBody, react_joint_moments};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallJoint {
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
}

impl BallJoint {
    pub fn world_anchors(&self, a: &RigidBody, b: &RigidBody) -> (Vec3, Vec3) {
        (a.position + a.rotation() * self.anchor_a, b.position + b.rotation() * self.anchor_b)
    }

    pub fn gap(&self, a: &RigidBody, b: &RigidBody) -> f32 {
        let (pa, pb) = self.world_anchors(a, b);
        pa.distance(pb)
    }

    pub fn fulcrum(&self, a: &RigidBody, b: &RigidBody) -> Vec3 {
        let (pa, pb) = self.world_anchors(a, b);
        if a.state == BodyState::Frozen {
            return pa;
        }
        if b.state == BodyState::Frozen {
            return pb;
        }
        let share = a.mass() / (a.mass() + b.mass());
        pa * share + pb * (1.0 - share)
    }

    /// `iterations` counts moment substeps, conventionally 32; geometry closes in this tick.
    pub fn solve(&self, a: &mut RigidBody, b: &mut RigidBody, dt: f32, iterations: usize) {
        if !dt.is_finite()
            || dt <= 0.0
            || iterations == 0
            || (a.state == BodyState::Frozen && b.state == BodyState::Frozen)
        {
            return;
        }
        let fulcrum = self.fulcrum(a, b);
        set_fulcrum(a, self.anchor_a, fulcrum);
        set_fulcrum(b, self.anchor_b, fulcrum);
        let mut ma = a.joint_moment(fulcrum);
        let mut mb = b.joint_moment(fulcrum);
        if react_joint_moments(&mut ma, &mut mb, fulcrum, iterations as u32) {
            a.apply_contact_moment(&ma);
            b.apply_contact_moment(&mb);
        }
    }
}

impl RigidBody {
    /// Original articulation point tensor; includes its diagonal parallel-axis contribution.
    pub fn joint_moment(&self, point: Vec3) -> super::ContactMoment {
        let mut moment =
            self.contact_moment(&BodyHit { immobile: self.state == BodyState::Frozen, ..Default::default() });
        let r = self.rotation().transpose() * (point - self.world_cog());
        let squared = r * r;
        let offset = Vec3::new(squared.y + squared.z, squared.x + squared.z, squared.x + squared.y) * self.mass();
        let inverse = super::inverse_diagonal(self.inertia() + offset);
        moment.inverse_inertia = self.rotation() * Mat3::from_diagonal(inverse) * self.rotation().transpose();
        moment
    }
}

fn set_fulcrum(body: &mut RigidBody, anchor: Vec3, fulcrum: Vec3) {
    if body.state == BodyState::Frozen {
        return;
    }
    let arm = body.rotation() * anchor;
    if let (Some(from), Some(to)) = (arm.try_normalize(), (fulcrum - body.position).try_normalize()) {
        body.set_orientation(Quat::from_rotation_arc(from, to) * body.orientation);
    }
    let position = fulcrum - body.rotation() * anchor;
    if position.distance_squared(body.position) > 0.000001 {
        body.wake();
    }
    body.position = position;
}
