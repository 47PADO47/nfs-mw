//! One rigid segment of the ragdoll: a capsule whose axis is its local Y axis, moved by position-based dynamics.

use glam::{Mat3, Quat, Vec3};

/// The largest linear speed (m/s) and angular speed (rad/s) a segment may have whatever it was thrown at: a guard against a
/// runaway solver. A fast bike is 90 m/s; a body that was thrown slower is held to a share of its own launch speed
/// ([`Body::cap`]).
pub(super) const MAX_SPEED: f32 = 100.0;
const MAX_SPIN: f32 = 60.0;

/// A segment of the ragdoll as the caller describes it: a capsule of `radius` whose end spheres are `half_length` from its centre
/// along its local Y axis. `position` is the centre of mass (the capsule's centre) and `orientation` the body frame, both in the
/// world, at the moment the ragdoll starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub mass: f32,
    pub radius: f32,
    pub half_length: f32,
    pub position: Vec3,
    pub orientation: Quat,
}

/// The state of a segment while it is simulated.
#[derive(Clone, Copy, Debug)]
pub(super) struct Body {
    pub x: Vec3,
    pub q: Quat,
    pub v: Vec3,
    pub w: Vec3,
    /// The pose at the start of the sub-step the solver is in (the friction and the velocities come from the move).
    pub x0: Vec3,
    pub q0: Quat,
    pub inv_mass: f32,
    /// The inverse principal moments of inertia in the body frame.
    pub inv_inertia: Vec3,
    pub radius: f32,
    pub half_length: f32,
    /// How much of the sub-step's move was recovery from an overlap (a push out of the ground or a wall beyond what the body's own
    /// motion into it needed): it is a correction of the place, not a motion, and gives the body no velocity (`settle`).
    pub recover: Vec3,
    /// The fastest the segment may move sideways or up (m/s).
    pub cap: f32,
}

impl Body {
    /// The inertia is that of a solid cylinder of radius `r` and the length of the whole capsule, `2 (half_length + r)`: about its
    /// axis `m r^2 / 2`, across it `m (3 r^2 + length^2) / 12`.
    pub fn new(segment: &Segment) -> Self {
        let mass = segment.mass.max(1e-3);
        let radius = segment.radius.max(1e-3);
        let length = 2.0 * (segment.half_length.max(0.0) + radius);
        let along = 0.5 * mass * radius * radius;
        let across = mass * (3.0 * radius * radius + length * length) / 12.0;
        Self {
            x: segment.position,
            q: segment.orientation.normalize(),
            v: Vec3::ZERO,
            w: Vec3::ZERO,
            x0: segment.position,
            q0: segment.orientation.normalize(),
            inv_mass: 1.0 / mass,
            inv_inertia: Vec3::new(1.0 / across, 1.0 / along, 1.0 / across),
            radius,
            half_length: segment.half_length.max(0.0),
            recover: Vec3::ZERO,
            cap: MAX_SPEED,
        }
    }

    /// The inverse inertia tensor in the world.
    pub fn inv_world(&self) -> Mat3 {
        let r = Mat3::from_quat(self.q);
        r * Mat3::from_diagonal(self.inv_inertia) * r.transpose()
    }

    /// A point given in the body frame, in the world.
    pub fn point(&self, local: Vec3) -> Vec3 {
        self.x + self.q * local
    }

    /// How little a push of direction `n` (unit) at the offset `r` from the centre moves the body: `1/m + (r x n) I^-1 (r x n)`.
    pub fn inverse_mass_at(&self, r: Vec3, n: Vec3) -> f32 {
        let c = r.cross(n);
        self.inv_mass + c.dot(self.inv_world() * c)
    }

    /// Move the body as a position-level impulse `p` applied at the offset `r` from its centre would.
    pub fn push_at(&mut self, r: Vec3, p: Vec3) {
        self.x += p * self.inv_mass;
        let turn = self.inv_world() * r.cross(p);
        self.rotate(turn);
    }

    /// Turn the body by the rotation vector `turn` (world frame).
    pub fn rotate(&mut self, turn: Vec3) {
        if !turn.is_finite() {
            return;
        }
        self.q = (Quat::from_scaled_axis(turn) * self.q).normalize();
    }

    /// The first half of a sub-step: gravity, damping, then the free motion.
    pub fn predict(&mut self, h: f32, gravity: f32, damping: (f32, f32)) {
        self.x0 = self.x;
        self.q0 = self.q;
        self.recover = Vec3::ZERO;
        self.v.y += gravity * h;
        self.v /= 1.0 + damping.0 * h;
        self.w /= 1.0 + damping.1 * h;
        self.x += self.v * h;
        self.rotate(self.w * h);
    }

    /// The last half: the velocities are what the solver's moves came to, less the recovery from overlaps (which is no motion: a
    /// body that was pushed out of a surface it was 40 cm into must not leave it at 40 cm per sub-step, 190 m/s).
    pub fn settle(&mut self, h: f32) {
        let moved = ((self.x - self.x0 - self.recover) / h).clamp_length_max(MAX_SPEED);
        // Falling is not capped (gravity may take a body from a bridge to 40 m/s); going sideways or up faster than the body was thrown is
        // a solver's launch, not a motion.
        let flat = Vec3::new(moved.x, 0.0, moved.z).clamp_length_max(self.cap);
        self.v = Vec3::new(flat.x, moved.y.min(self.cap), flat.z);
        let mut turn = self.q * self.q0.inverse();
        if turn.w < 0.0 {
            turn = -turn;
        }
        // The turn as an axis and an angle, with the angle from `atan2` (the `acos` of a quaternion near the identity is too coarse).
        let sine = Vec3::new(turn.x, turn.y, turn.z).length();
        self.w = match sine > 1e-9 {
            true => {
                (Vec3::new(turn.x, turn.y, turn.z) * (2.0 * sine.atan2(turn.w) / sine / h)).clamp_length_max(MAX_SPIN)
            }
            false => Vec3::ZERO,
        };
    }
}
