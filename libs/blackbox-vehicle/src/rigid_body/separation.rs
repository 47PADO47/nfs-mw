//! Penetration correction before the object impulse solve.

use super::{BodyContact, ContactMoment, RigidBody, Separation};

const EPSILON: f32 = 0.000001;

pub(super) fn correct(
    a: &mut RigidBody,
    b: &mut RigidBody,
    ma: &mut ContactMoment,
    mb: &mut ContactMoment,
    contact: &mut BodyContact,
    mode: Separation,
) -> bool {
    if !contact.overlap.is_finite() || contact.overlap <= EPSILON || !contact.point.is_finite() {
        return false;
    }
    let normal = contact.normal;
    if !normal.is_finite() || normal.length_squared() <= EPSILON {
        return false;
    }
    if ma.immobile && mb.immobile {
        for moment in [&mut *ma, &mut *mb] {
            if moment.breaking_impulse <= 0.0 {
                continue;
            }
            moment.immobile = false;
            moment.breaking_impulse = 0.0;
        }
    }
    if ma.immobile && mb.immobile {
        return false;
    }
    let share = match (ma.immobile, mb.immobile) {
        (true, _) => 0.0,
        (_, true) => 1.0,
        _ => mobile_share(a, b, normal, mode),
    };
    let delta_a = normal * (share * contact.overlap);
    let delta_b = normal * (-(1.0 - share) * contact.overlap);
    a.position += delta_a;
    b.position += delta_b;
    ma.centre += delta_a;
    mb.centre += delta_b;
    contact.point += match contact.point_on_first {
        true => delta_a,
        false => delta_b,
    };
    true
}

fn mobile_share(a: &RigidBody, b: &RigidBody, normal: glam::Vec3, mode: Separation) -> f32 {
    if mode == Separation::Mass {
        return b.mass() / (a.mass() + b.mass());
    }
    let va = -a.linear_velocity.dot(normal);
    let vb = b.linear_velocity.dot(normal);
    let closing = va + vb;
    if closing.abs() <= EPSILON {
        return b.mass() / (a.mass() + b.mass());
    }
    let own = match closing > 0.0 {
        true => va,
        false => vb,
    };
    (own / closing).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests;
