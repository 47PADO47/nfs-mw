//! A ball joint between two rigid bodies: one point of each is held on the same place in the world, the
//! bodies are free to turn about it. Used to hitch a trailer to a tractor (the "5th wheel").
//!
//! A sequential-impulse constraint, as in any rigid body engine: the relative velocity of the two anchor
//! points is driven to zero by impulses, with a Baumgarte term that turns the remaining distance between
//! the anchors into a closing speed so numerical drift does not accumulate.

use glam::{Mat3, Vec3};

use super::body::{BodyState, RigidBody};

/// The Baumgarte factor: the share of the anchor gap the correction closes in one step.
pub const BAUMGARTE: f32 = 0.2;
/// Two sleeping bodies whose anchors are this close (m) are left asleep.
const SLEEPING_GAP: f32 = 0.001;
/// Largest closing speed (m/s) the position correction may ask for, so a badly misplaced joint pulls
/// together gently instead of throwing the bodies.
pub const MAX_CORRECTION_SPEED: f32 = 4.0;

/// A joint tying `anchor_a` of one body to `anchor_b` of another. Anchors are points in each body's own
/// frame, measured from its origin (`RigidBody::position`), in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallJoint {
    pub anchor_a: Vec3,
    pub anchor_b: Vec3,
}

impl BallJoint {
    /// The two anchor points in the world, in the order of the bodies.
    pub fn world_anchors(&self, a: &RigidBody, b: &RigidBody) -> (Vec3, Vec3) {
        (a.position + a.rotation() * self.anchor_a, b.position + b.rotation() * self.anchor_b)
    }

    /// Distance between the two anchors in the world: zero when the joint holds.
    pub fn gap(&self, a: &RigidBody, b: &RigidBody) -> f32 {
        let (pa, pb) = self.world_anchors(a, b);
        pa.distance(pb)
    }

    /// Applies impulses so the anchors move together, `iterations` times over (one is exact for a single
    /// joint, more only matter when several constraints share a body). `dt` is the step in seconds.
    pub fn solve(&self, a: &mut RigidBody, b: &mut RigidBody, dt: f32, iterations: usize) {
        if dt <= 0.0 || (a.state == BodyState::Frozen && b.state == BodyState::Frozen) {
            return;
        }
        if a.state == BodyState::Asleep && b.state == BodyState::Asleep && self.gap(a, b) < SLEEPING_GAP {
            return;
        }
        for _ in 0..iterations {
            let (pa, pb) = self.world_anchors(a, b);
            let (ra, rb) = (pa - a.world_cog(), pb - b.world_cog());
            let gap = pa - pb;
            let closing = (-BAUMGARTE / dt * gap).clamp_length_max(MAX_CORRECTION_SPEED);
            let relative = a.point_velocity(pa) - b.point_velocity(pb);
            let k = effective_mass_inverse(a, ra) + effective_mass_inverse(b, rb);
            // Two bodies that cannot move (no mass, or frozen) make the matrix singular: nothing to solve.
            let inverse = k.inverse();
            if !inverse.is_finite() {
                return;
            }
            let impulse = inverse * (closing - relative);
            a.apply_impulse_at(impulse, pa);
            b.apply_impulse_at(-impulse, pb);
        }
    }
}

/// The matrix that turns an impulse at offset `r` from the centre of gravity into the change of velocity of
/// that point: `1/m I - [r] I^-1 [r]`. A frozen body does not move, so it adds nothing.
fn effective_mass_inverse(body: &RigidBody, r: Vec3) -> Mat3 {
    if body.state == BodyState::Frozen {
        return Mat3::ZERO;
    }
    let skew = Mat3::from_cols(Vec3::new(0.0, r.z, -r.y), Vec3::new(-r.z, 0.0, r.x), Vec3::new(r.y, -r.x, 0.0));
    Mat3::from_diagonal(Vec3::splat(body.inv_mass)) - skew * body.inv_world_tensor() * skew
}
