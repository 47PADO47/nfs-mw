//! Iterated response of two collision moments. Spec: `docs/specs/pc-object-contact.md`.
//! Moments are temporary collision parameters; changing one never retunes a vehicle's suspension.

use glam::{Mat3, Vec3};

use super::{FrictionState, PlaneContact, Reaction, transform_vector};

/// One body's effective parameters for an object contact, all in world space.
#[derive(Clone, Copy, Debug)]
pub struct ContactMoment {
    pub mass: f32,
    pub inverse_inertia: Mat3,
    pub centre: Vec3,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub elasticity: f32,
    pub immobile: bool,
    /// Positive threshold for releasing an immobile body, in N s; zero means unbreakable.
    pub breaking_impulse: f32,
    /// The centre stays fixed while the body can rotate.
    pub fixed_centre: bool,
}

impl ContactMoment {
    /// Collision callback intensity. An immobile moment uses its partner's contact mass.
    /// This can be positive even when this moment receives no physical velocity change.
    pub fn notification_delta_v(&self, other: &Self, impulse: f32) -> f32 {
        let mass = match self.immobile {
            true => other.mass,
            false => self.mass,
        };
        match mass > 0.0 {
            true => impulse / mass,
            false => 0.0,
        }
    }

    fn is_valid(&self) -> bool {
        self.mass.is_finite()
            && self.mass > 0.0
            && self.inverse_inertia.is_finite()
            && self.centre.is_finite()
            && self.linear_velocity.is_finite()
            && self.angular_velocity.is_finite()
            && self.elasticity.is_finite()
            && self.breaking_impulse.is_finite()
    }
}

struct IterationBody {
    linear: Vec3,
    angular: Vec3,
    output_linear: Vec3,
    output_angular: Vec3,
    arm: Vec3,
    immobile: bool,
}

impl IterationBody {
    fn new(moment: &ContactMoment, point: Vec3, immobile: bool) -> Self {
        Self {
            linear: moment.linear_velocity,
            angular: moment.angular_velocity,
            output_linear: Vec3::ZERO,
            output_angular: Vec3::ZERO,
            arm: point - moment.centre,
            immobile,
        }
    }

    fn velocity(&self) -> Vec3 {
        self.linear + self.angular.cross(self.arm)
    }

    fn release(&mut self, moment: &mut ContactMoment, impulse: f32) {
        if moment.breaking_impulse > 0.0 && impulse > moment.breaking_impulse {
            self.immobile = false;
            moment.immobile = false;
        }
    }

    fn apply(&mut self, moment: &ContactMoment, normal: Vec3, tangent: Vec3) {
        if self.immobile {
            return;
        }
        let turn_normal = transform_vector(moment.inverse_inertia, self.arm.cross(normal));
        let turn_tangent = transform_vector(moment.inverse_inertia, self.arm.cross(tangent));
        let turn = turn_normal + turn_tangent;
        self.angular += turn;
        self.output_angular += turn + turn_normal * moment.elasticity;
        if moment.fixed_centre {
            return;
        }
        let inv_mass = moment.mass.recip();
        let normal = normal * inv_mass;
        let tangent = tangent * inv_mass;
        let linear = normal + tangent;
        self.linear += linear;
        self.output_linear += linear + normal * moment.elasticity;
    }

    fn finish(self, moment: &mut ContactMoment) {
        moment.linear_velocity += self.output_linear;
        moment.angular_velocity += self.output_angular;
    }
}

fn reduced_mass(a: &ContactMoment, b: &ContactMoment, immobile_a: bool, immobile_b: bool) -> f32 {
    if immobile_a {
        return b.mass;
    }
    if immobile_b {
        return a.mass;
    }
    a.mass * b.mass / (a.mass + b.mass)
}

/// Articulation opposes the full point velocity without friction, approach filtering or restitution.
pub fn react_joint_moments(a: &mut ContactMoment, b: &mut ContactMoment, point: Vec3, iterations: u32) -> bool {
    if iterations == 0 || !a.is_valid() || !b.is_valid() || !point.is_finite() || (a.immobile && b.immobile) {
        return false;
    }
    let (mut left, mut right) = (IterationBody::new(a, point, a.immobile), IterationBody::new(b, point, b.immobile));
    let mass = reduced_mass(a, b, a.immobile, b.immobile);
    let mut reacted = false;
    let (mut ma, mut mb) = (*a, *b);
    ma.elasticity = 0.0;
    mb.elasticity = 0.0;
    for i in 0..iterations {
        let momentum = (left.velocity() - right.velocity()) * mass;
        if momentum.length_squared() == 0.0 {
            break;
        }
        reacted = true;
        if i == 0 {
            continue;
        }
        let impulse = -momentum * (i as f32 / iterations as f32);
        left.apply(&ma, impulse, Vec3::ZERO);
        right.apply(&mb, -impulse, Vec3::ZERO);
    }
    if reacted {
        left.finish(a);
        right.finish(b);
    }
    reacted
}

fn friction(momentum: Vec3, normal: Vec3, impulse: f32, plane: &PlaneContact) -> (Vec3, FrictionState) {
    let tangent = momentum + normal * impulse;
    let length = tangent.length();
    if length > plane.friction_static * impulse {
        return (-tangent * (plane.friction_kinetic * impulse / length), FrictionState::Dynamic);
    }
    (-tangent, FrictionState::Static)
}

/// Resolve a contact using separate temporary velocities and restitution increments.
/// Object contacts conventionally use 32 iterations. Returns the initial closing/sliding speed,
/// initial friction state and accumulated normal impulse, before restitution.
/// An invalid input or an initially separating/stationary contact leaves both moments unchanged.
pub fn react_moments(
    a: &mut ContactMoment,
    b: &mut ContactMoment,
    plane: &PlaneContact,
    iterations: u32,
) -> Option<Reaction> {
    if iterations == 0 || !a.is_valid() || !b.is_valid() || !plane.point.is_finite() {
        return None;
    }
    if !plane.friction_static.is_finite() || !plane.friction_kinetic.is_finite() {
        return None;
    }
    let normal = plane.normal.try_normalize()?;
    let (mut left, mut right) = (
        IterationBody::new(a, plane.point, a.immobile && !b.immobile),
        IterationBody::new(b, plane.point, b.immobile && !a.immobile),
    );
    let mut mass = reduced_mass(a, b, left.immobile, right.immobile);
    let mut reaction = None;
    for i in 0..iterations {
        let velocity = left.velocity() - right.velocity();
        let momentum = velocity * mass;
        let approach = -((momentum.x * normal.x + momentum.z * normal.z) + momentum.y * normal.y);
        if approach <= 0.0 {
            break;
        }
        let (tangent, state) = friction(momentum, normal, approach, plane);
        if i == 0 {
            let closing_speed = -velocity.dot(normal);
            reaction = Some(Reaction {
                impulse: 0.0,
                closing_speed,
                sliding_speed: (velocity + normal * closing_speed).length(),
                friction: state,
            });
            left.release(a, approach);
            right.release(b, approach);
            mass = reduced_mass(a, b, left.immobile, right.immobile);
            continue;
        }
        let fraction = i as f32 / iterations as f32;
        let normal_impulse = (normal * approach) * fraction;
        let tangent_impulse = tangent * fraction;
        left.apply(a, normal_impulse, tangent_impulse);
        right.apply(b, -normal_impulse, -tangent_impulse);
        if let Some(result) = reaction.as_mut() {
            result.impulse += approach * fraction;
        }
    }
    left.finish(a);
    right.finish(b);
    reaction
}
