//! Damage-zone selection and the pre-contact data retained by the original callback.

use blackbox_vehicle::rigid_body::RigidBody;
use glam::Vec3;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OtherBody {
    pub vehicle: bool,
    pub player: bool,
    pub cop: bool,
    pub no_car_effect: bool,
    /// Source smackable mass and immobility, independent of temporary contact scaling.
    pub smackable: Option<(f32, bool)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageHit {
    pub arm: Vec3,
    pub normal: Vec3,
    pub half: Vec3,
    pub closing_speed: f32,
    pub force: f32,
    pub shock_force: f32,
    pub before_velocity: Vec3,
    pub other: OtherBody,
}

impl DamageHit {
    pub fn capture(body: &RigidBody, point: Vec3, normal: Vec3, closing_speed: f32, other: OtherBody) -> Self {
        let inverse = body.rotation().transpose();
        Self {
            arm: inverse * (point - body.position),
            normal: inverse * normal,
            half: body.dimension(),
            closing_speed,
            force: 0.0,
            shock_force: 0.0,
            before_velocity: body.linear_velocity,
            other,
        }
    }

    /// Original normal-impulse callback, excluding restitution and tangential friction.
    pub fn resolved(mut self, body: &RigidBody, normal_delta_v: f32, object: bool) -> Self {
        self.force = normal_delta_v * body.mass();
        if object {
            self.force *= 2.0;
        }
        self
    }

    /// Cop OnImpact eligibility for health and visuals; object shock is applied before this filter.
    pub fn allows_cop(&self) -> bool {
        !self.other.no_car_effect && !self.other.smackable.is_some_and(|(mass, immobile)| !immobile && mass <= 2000.0)
    }

    /// Order matches DAMAGE_ZONES, including strict half-extent/direction thresholds.
    pub fn zone(&self) -> usize {
        let side = |right, left, middle| {
            if self.arm.x > self.half.x * 0.5 && self.normal.x < -0.8 {
                return right;
            }
            if self.arm.x < -self.half.x * 0.5 && self.normal.x > 0.8 {
                return left;
            }
            middle
        };
        if self.arm.z > self.half.z * 0.5 {
            return side(5, 4, 0);
        }
        if self.arm.z < -self.half.z * 0.5 {
            return side(7, 6, 1);
        }
        if self.arm.y > self.half.y * 0.5 && self.normal.y < -0.8 {
            return 8;
        }
        if self.arm.y < -self.half.y * 0.5 && self.normal.y > 0.8 {
            return 9;
        }
        if self.arm.x > 0.0 {
            return 3;
        }
        2
    }
}
