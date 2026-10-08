use glam::Vec3;

use super::body::{BodyState, RigidBody};

/// A contact between the body and an immovable surface.
#[derive(Clone, Copy, Debug)]
pub struct PlaneContact {
    /// World point where the body touches the surface.
    pub point: Vec3,
    /// Unit normal pointing from the surface toward the body.
    pub normal: Vec3,
    /// Static and kinetic friction coefficients of the pair.
    pub friction_static: f32,
    pub friction_kinetic: f32,
}

/// Per-hit tuning of the impulse.
#[derive(Clone, Copy, Debug)]
pub struct ContactParams {
    /// Restitution (0 = no bounce, 1 = elastic).
    pub restitution: f32,
    /// Multiplier on the principal moments, per body axis (right, up, forward); `> 1` resists rotation.
    pub inertia_scale: Vec3,
}

/// What friction did in a reaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrictionState {
    None,
    Static,
    Dynamic,
}

/// Outcome of a resolved contact.
#[derive(Clone, Copy, Debug)]
pub struct Reaction {
    /// Magnitude of the normal impulse (N s), usable as a collision force for events.
    pub impulse: f32,
    /// Approach speed along the normal before the hit (m/s).
    pub closing_speed: f32,
    /// Tangential speed of the contact point before friction (m/s).
    pub sliding_speed: f32,
    pub friction: FrictionState,
}

impl RigidBody {
    /// Resolves a contact with an immovable surface by a classical impulse with Coulomb friction. Updates
    /// the velocities and returns `None` when the contact point is not approaching the surface.
    ///
    /// The original solver is not specified; this is the textbook rule (see the README).
    pub fn react_plane(&mut self, contact: &PlaneContact, params: &ContactParams) -> Option<Reaction> {
        if self.state == BodyState::Frozen {
            return None;
        }
        self.wake();
        let n = contact.normal;
        let inv_tensor = self.scaled_inv_world_tensor(params.inertia_scale);
        let r = contact.point - self.world_cog();
        let v_point = self.linear_velocity + self.angular_velocity.cross(r);
        let vn = v_point.dot(n);
        if vn >= 0.0 {
            return None;
        }
        let effective_mass_inv = |dir: Vec3| self.inv_mass + dir.dot((inv_tensor * r.cross(dir)).cross(r));
        let jn = -(1.0 + params.restitution.clamp(0.0, 1.0)) * vn / effective_mass_inv(n);
        let mut v = self.linear_velocity + n * (jn * self.inv_mass);
        let mut w = self.angular_velocity + inv_tensor * r.cross(n * jn);

        // Coulomb friction against the tangential velocity left after the normal impulse.
        let vp = v + w.cross(r);
        let vt = vp - n * vp.dot(n);
        let sliding = vt.length();
        let mut friction = FrictionState::None;
        if sliding > 1e-6 {
            let t = vt / sliding;
            let jt_stop = sliding / effective_mass_inv(t);
            let (jt, state) = if jt_stop <= contact.friction_static * jn {
                (jt_stop, FrictionState::Static)
            } else {
                (contact.friction_kinetic * jn, FrictionState::Dynamic)
            };
            v -= t * (jt * self.inv_mass);
            w -= inv_tensor * r.cross(t * jt);
            friction = state;
        }
        self.linear_velocity = v;
        self.angular_velocity = w;
        Some(Reaction { impulse: jn, closing_speed: -vn, sliding_speed: sliding, friction })
    }
}
