//! Two bodies against each other: separating them and the impulse that changes their velocities.
//! Spec: `docs/specs/pc-object-contact.md`. Response uses the measured PC iteration rule;
//! penetration correction follows `docs/specs/box-object-contacts.md`.

use glam::Vec3;

use super::body::{BodyState, RigidBody};
use super::{ContactMoment, PlaneContact, Reaction, react_moments};

/// What differs per body in a hit: how bouncy it is here and how hard it is to turn.
#[derive(Clone, Copy, Debug)]
pub struct BodyHit {
    pub restitution: f32,
    pub inertia_scale: Vec3,
    pub mass_scale: f32,
    /// World-space contact centre; `None` uses the body's stored centre of gravity.
    pub centre: Option<Vec3>,
    pub immobile: bool,
    pub breaking_impulse: f32,
    pub fixed_centre: bool,
}

impl Default for BodyHit {
    fn default() -> Self {
        Self {
            restitution: 0.0,
            inertia_scale: Vec3::ONE,
            mass_scale: 1.0,
            centre: None,
            immobile: false,
            breaking_impulse: 0.0,
            fixed_centre: false,
        }
    }
}

/// A contact between two bodies.
#[derive(Clone, Copy, Debug)]
pub struct BodyContact {
    pub point: Vec3,
    /// Unit normal from the second body to the first.
    pub normal: Vec3,
    pub overlap: f32,
    pub friction_static: f32,
    pub friction_kinetic: f32,
    pub point_on_first: bool,
}

/// Which rule a caller uses to distribute overlap between two mobile bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Separation {
    Mass,
    NormalSpeed,
}

/// The response and each body's effective collision notification, in m/s.
pub struct BodyReaction {
    pub reaction: Reaction,
    pub immobile: [bool; 2],
    pub impulse_delta_v: [f32; 2],
    /// Collision callback intensity. An immobile moment uses its partner's effective mass,
    /// even though it receives no physical velocity change.
    pub notification_delta_v: [f32; 2],
}

impl BodyContact {
    pub fn from_geometry(contact: super::ObbContact, friction: [f32; 2]) -> Self {
        Self {
            point: contact.point,
            normal: contact.normal,
            overlap: contact.overlap,
            friction_static: friction[0],
            friction_kinetic: friction[1],
            point_on_first: contact.point_on_first,
        }
    }
}

/// Select moments before separation, update their world centres and the incident surface point,
/// then resolve the velocities. Body positions can change even without an approaching contact.
pub fn resolve_bodies(
    a: &mut RigidBody,
    b: &mut RigidBody,
    contact: &BodyContact,
    hit_a: &BodyHit,
    hit_b: &BodyHit,
    separation: Separation,
) -> Option<BodyReaction> {
    if a.state == BodyState::Frozen || b.state == BodyState::Frozen {
        return None;
    }
    let mut left = a.contact_moment(hit_a);
    let mut right = b.contact_moment(hit_b);
    let mut contact = *contact;
    if !super::separation::correct(a, b, &mut left, &mut right, &mut contact, separation) {
        return None;
    }
    let plane = PlaneContact {
        point: contact.point,
        normal: contact.normal,
        friction_static: contact.friction_static,
        friction_kinetic: contact.friction_kinetic,
    };
    let reaction = react_moments(&mut left, &mut right, &plane, 32)?;
    a.apply_contact_moment(&left);
    b.apply_contact_moment(&right);
    let impulse_delta_v = [&left, &right].map(|moment| match moment.immobile {
        true => 0.0,
        false => reaction.impulse / moment.mass,
    });
    let notification_delta_v =
        [(&left, &right), (&right, &left)].map(|(own, other)| own.notification_delta_v(other, reaction.impulse));
    Some(BodyReaction { reaction, immobile: [left.immobile, right.immobile], impulse_delta_v, notification_delta_v })
}

/// Pushes two overlapping bodies apart along the normal, the lighter one further (by the mass ratio).
pub fn separate(a: &mut RigidBody, b: &mut RigidBody, normal: Vec3, overlap: f32) {
    let total = a.mass + b.mass;
    let move_b = a.mass / total;
    a.position += normal * (overlap * (1.0 - move_b));
    b.position -= normal * (overlap * move_b);
}

/// Resolve a pair with 32 iterated impulses. Each body keeps its own restitution.
/// Effective masses and centres exist only for this contact and never retune the stored bodies.
pub fn react_bodies(
    a: &mut RigidBody,
    b: &mut RigidBody,
    contact: &BodyContact,
    hit_a: &BodyHit,
    hit_b: &BodyHit,
) -> Option<Reaction> {
    if a.state == BodyState::Frozen || b.state == BodyState::Frozen {
        return None;
    }
    let mut left = a.contact_moment(hit_a);
    let mut right = b.contact_moment(hit_b);
    let plane = PlaneContact {
        point: contact.point,
        normal: contact.normal,
        friction_static: contact.friction_static,
        friction_kinetic: contact.friction_kinetic,
    };
    let reaction = react_moments(&mut left, &mut right, &plane, 32)?;
    a.apply_contact_moment(&left);
    b.apply_contact_moment(&right);
    Some(reaction)
}

impl RigidBody {
    /// Create temporary parameters without modifying integration mass, inertia or centre of gravity.
    pub fn contact_moment(&self, hit: &BodyHit) -> ContactMoment {
        ContactMoment {
            mass: self.mass * hit.mass_scale,
            inverse_inertia: self.scaled_inv_world_tensor(hit.inertia_scale),
            centre: hit.centre.unwrap_or_else(|| self.world_cog()),
            linear_velocity: self.linear_velocity,
            angular_velocity: self.angular_velocity,
            elasticity: hit.restitution,
            immobile: hit.immobile,
            breaking_impulse: hit.breaking_impulse,
            fixed_centre: hit.fixed_centre,
        }
    }

    /// Commit only contact velocities. Attachment state belongs to the caller.
    pub fn apply_contact_moment(&mut self, moment: &ContactMoment) {
        if self.state == BodyState::Frozen || moment.immobile {
            return;
        }
        self.wake();
        self.linear_velocity = moment.linear_velocity;
        self.angular_velocity = moment.angular_velocity;
    }
}
