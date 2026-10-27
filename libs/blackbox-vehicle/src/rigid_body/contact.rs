use glam::{Mat3, Vec3};

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
    /// Resolves an immovable plane with 16 moment iterations. Restitution affects the output
    /// velocities while the reported normal impulse excludes restitution.
    pub fn react_plane(&mut self, contact: &PlaneContact, params: &ContactParams) -> Option<Reaction> {
        self.react_plane_at(contact, params, None)
    }

    /// As [`Self::react_plane`], with the lever arm taken from `centre` instead of the centre of gravity.
    pub fn react_plane_at(
        &mut self,
        contact: &PlaneContact,
        params: &ContactParams,
        centre: Option<Vec3>,
    ) -> Option<Reaction> {
        if self.state == BodyState::Frozen {
            return None;
        }
        let mut body = self.contact_moment(&super::BodyHit {
            restitution: params.restitution,
            inertia_scale: params.inertia_scale,
            centre,
            ..Default::default()
        });
        let mut surface = super::ContactMoment {
            mass: 1.0,
            inverse_inertia: Mat3::ZERO,
            centre: contact.point,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            elasticity: 0.0,
            immobile: true,
            breaking_impulse: 0.0,
            fixed_centre: true,
        };
        let reaction = super::moment::react_plane_moments(&mut body, &mut surface, contact, 16)?;
        self.apply_contact_moment(&body);
        Some(reaction)
    }
}
