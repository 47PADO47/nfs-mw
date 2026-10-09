use glam::{Mat3, Quat, Vec3};

use super::inertia::{box_inertia, inverse_diagonal};
use super::spec::RigidBodySpec;

/// Largest linear speed per axis (m/s).
pub const MAX_LINEAR_SPEED: f32 = 300.0;
/// Largest angular speed per axis (rad/s).
pub const MAX_ANGULAR_SPEED: f32 = 30.0;
/// Air density used by the drag terms (kg/m^3).
const AIR_DENSITY: f32 = 1.225;

/// Whether a body is simulated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyState {
    Awake,
    /// Not integrated, velocities zero; any applied force wakes it.
    Asleep,
    /// Frozen: no integration, no collisions.
    Frozen,
}

/// One rigid body: position and orientation of its origin, velocities of its centre of gravity, force and
/// torque accumulators. Forces added during a step are integrated by the next [`RigidBody::begin_frame`]
/// (semi-implicit Euler: velocity first, then position from the new velocity).
#[derive(Clone, Debug)]
pub struct RigidBody {
    pub position: Vec3,
    pub orientation: Quat,
    pub linear_velocity: Vec3,
    /// World-frame angular velocity (rad/s).
    pub angular_velocity: Vec3,
    pub state: BodyState,
    pub(super) mass: f32,
    pub(super) inv_mass: f32,
    pub(super) inertia: Vec3,
    pub(super) dimension: Vec3,
    pub(super) radius: f32,
    pub(super) cog: Vec3,
    pub(super) force: Vec3,
    pub(super) torque: Vec3,
    rotation: Mat3,
    inv_world_tensor: Mat3,
    pub(super) spec: RigidBodySpec,
}

impl RigidBody {
    /// A body of `mass` kg with the given half extents (m); the inertia is that of a solid box scaled per
    /// axis by `tensor_scale`.
    pub fn new(mass: f32, dimension: Vec3, tensor_scale: Vec3, spec: RigidBodySpec) -> Self {
        let dimension = dimension.max(Vec3::splat(0.001));
        let mass = mass.max(f32::EPSILON);
        let mut body = Self {
            position: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            linear_velocity: Vec3::ZERO,
            angular_velocity: Vec3::ZERO,
            state: BodyState::Awake,
            mass,
            inv_mass: 1.0 / mass,
            inertia: box_inertia(mass, dimension) * tensor_scale,
            dimension,
            radius: dimension.length(),
            cog: spec.cg,
            force: Vec3::ZERO,
            torque: Vec3::ZERO,
            rotation: Mat3::IDENTITY,
            inv_world_tensor: Mat3::IDENTITY,
            spec,
        };
        body.refresh_matrices();
        body
    }

    pub fn mass(&self) -> f32 {
        self.mass
    }

    /// Principal moments of inertia in the body frame (kg m^2).
    pub fn inertia(&self) -> Vec3 {
        self.inertia
    }

    /// Half extents of the body box (m).
    pub fn dimension(&self) -> Vec3 {
        self.dimension
    }

    /// Bounding radius, `|dimension|`.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn spec(&self) -> &RigidBodySpec {
        &self.spec
    }

    /// Centre of gravity offset in the body frame.
    pub fn cog(&self) -> Vec3 {
        self.cog
    }

    /// Moves the centre of gravity (body frame). The body origin stays where it is.
    pub fn set_cog(&mut self, cog: Vec3) {
        self.cog = cog;
    }

    /// Changes the mass, rescaling the inertia by `new / old`. Ignored if not positive or unchanged.
    pub fn set_mass(&mut self, mass: f32) {
        if mass > 0.0 && mass != self.mass {
            self.inertia *= mass / self.mass;
            self.mass = mass;
            self.inv_mass = 1.0 / mass;
            self.refresh_matrices();
        }
    }

    /// Rotation from the body frame to the world (columns are the world right, up and forward axes).
    pub fn rotation(&self) -> Mat3 {
        self.rotation
    }

    /// Cached inverse world inertia tensor.
    pub fn inv_world_tensor(&self) -> Mat3 {
        self.inv_world_tensor
    }

    /// World position of the centre of gravity.
    pub fn world_cog(&self) -> Vec3 {
        self.position + self.rotation * self.cog
    }

    /// Velocity of a world point attached to the body.
    pub fn point_velocity(&self, world_point: Vec3) -> Vec3 {
        self.linear_velocity + self.angular_velocity.cross(world_point - self.world_cog())
    }

    /// The unapplied force accumulator (world frame).
    pub fn pending_force(&self) -> Vec3 {
        self.force
    }

    /// The unapplied torque accumulator (world frame).
    pub fn pending_torque(&self) -> Vec3 {
        self.torque
    }

    fn refresh_matrices(&mut self) {
        self.rotation = Mat3::from_quat(self.orientation);
        let inv = Mat3::from_diagonal(inverse_diagonal(self.inertia));
        self.inv_world_tensor = self.rotation * inv * self.rotation.transpose();
    }

    /// Places the body: awake, given pose, velocities and accumulators cleared.
    pub fn place(&mut self, position: Vec3, orientation: Quat) {
        self.position = position;
        self.orientation = orientation.normalize();
        self.linear_velocity = Vec3::ZERO;
        self.angular_velocity = Vec3::ZERO;
        self.force = Vec3::ZERO;
        self.torque = Vec3::ZERO;
        self.state = BodyState::Awake;
        self.refresh_matrices();
    }

    pub fn wake(&mut self) {
        if self.state == BodyState::Asleep {
            self.state = BodyState::Awake;
        }
    }

    /// Adds a force (N, world frame) at the centre of gravity. Wakes a sleeping body.
    pub fn apply_force(&mut self, force: Vec3) {
        self.wake();
        self.force += force;
    }

    /// Adds a torque (N m, world frame). Wakes a sleeping body.
    pub fn apply_torque(&mut self, torque: Vec3) {
        self.wake();
        self.torque += torque;
    }

    /// Adds a force acting at a world point: force plus the torque about the centre of gravity.
    pub fn apply_force_at(&mut self, force: Vec3, world_point: Vec3) {
        self.wake();
        self.torque += (world_point - self.world_cog()).cross(force);
        self.force += force;
    }

    /// Changes the velocities as an impulse (N s, world frame) at a world point would: the linear velocity by
    /// `impulse / mass` and the angular velocity by the torque of the impulse about the centre of gravity.
    /// Wakes a sleeping body; a frozen one does not move.
    pub fn apply_impulse_at(&mut self, impulse: Vec3, world_point: Vec3) {
        if self.state == BodyState::Frozen {
            return;
        }
        self.wake();
        self.linear_velocity += impulse * self.inv_mass;
        self.angular_velocity += self.inv_world_tensor * (world_point - self.world_cog()).cross(impulse);
    }

    /// Adds only the torque of a force acting at a world point.
    pub fn apply_torque_at(&mut self, force: Vec3, world_point: Vec3) {
        self.wake();
        self.torque += (world_point - self.world_cog()).cross(force);
    }

    /// Adds `accel * dt` to the linear velocity (awake bodies only).
    pub fn accelerate(&mut self, accel: Vec3, dt: f32) {
        if self.state == BodyState::Awake {
            self.linear_velocity += accel * dt;
        }
    }

    /// Multiplies velocities and the pending force and torque by `1 - k`.
    pub fn damp(&mut self, k: f32) {
        let s = 1.0 - k;
        self.linear_velocity *= s;
        self.angular_velocity *= s;
        self.force *= s;
        self.torque *= s;
    }

    /// Integrates the previous step's force and torque, clears the accumulators and adds gravity.
    pub fn begin_frame(&mut self, dt: f32) {
        match self.state {
            BodyState::Frozen => return,
            BodyState::Asleep => {
                self.linear_velocity = Vec3::ZERO;
                self.angular_velocity = Vec3::ZERO;
                self.force = Vec3::ZERO;
                self.torque = Vec3::ZERO;
                return;
            }
            BodyState::Awake => {}
        }
        let cg0 = self.rotation * self.cog;
        self.linear_velocity += self.force * (dt * self.inv_mass);
        self.position += self.linear_velocity * dt;
        self.angular_velocity += self.inv_world_tensor * (self.torque * dt);
        self.linear_velocity =
            self.linear_velocity.clamp(Vec3::splat(-MAX_LINEAR_SPEED), Vec3::splat(MAX_LINEAR_SPEED));
        self.angular_velocity =
            self.angular_velocity.clamp(Vec3::splat(-MAX_ANGULAR_SPEED), Vec3::splat(MAX_ANGULAR_SPEED));
        if self.angular_velocity.length_squared() != 0.0 {
            let w = self.angular_velocity * dt;
            let dq = Quat::from_xyzw(w.x, w.y, w.z, 0.0) * self.orientation;
            self.orientation = (self.orientation + dq * 0.5).normalize();
        }
        self.refresh_matrices();
        // Rotate about the centre of gravity, not about the origin.
        self.position += cg0 - self.rotation * self.cog;
        self.force = Vec3::ZERO;
        self.torque = Vec3::ZERO;
        self.force.y += self.spec.gravity * self.mass;
    }

    /// Adds the quadratic linear and angular drag of the body box to the accumulators (integrated next
    /// step). Skipped for bodies that are not awake.
    pub fn apply_drag(&mut self) {
        if self.state != BodyState::Awake {
            return;
        }
        let h = self.dimension;
        let rot = self.rotation;
        if self.spec.drag != Vec3::ZERO {
            let v = rot.transpose() * self.linear_velocity;
            let area = 4.0 * h * h;
            let f = -0.5 * AIR_DENSITY * v.length() * (v * area * self.spec.drag);
            self.force += rot * f;
        }
        if self.spec.drag_angular != Vec3::ZERO {
            let w = rot.transpose() * self.angular_velocity;
            let s = Vec3::new(
                4.0 * h.x * (h.y * h.y + h.z * h.z).sqrt(),
                4.0 * h.y * (h.x * h.x + h.z * h.z).sqrt(),
                4.0 * h.z * (h.x * h.x + h.y * h.y).sqrt(),
            );
            let t = -0.5 * AIR_DENSITY * w.length() * (w * s * self.spec.drag_angular);
            self.torque += rot * t;
        }
    }

    /// Puts the body to sleep when it is slow and touches enough points. Returns the new state.
    pub fn update_sleep(&mut self, contact_points: usize) -> BodyState {
        if self.state == BodyState::Awake {
            let sum = self.linear_velocity.length() + self.angular_velocity.length() * self.radius;
            if sum < self.spec.sleep_velocity && contact_points > 2 {
                self.state = BodyState::Asleep;
                self.linear_velocity = Vec3::ZERO;
                self.angular_velocity = Vec3::ZERO;
            }
        }
        self.state
    }

    /// Inverse world inertia with the principal moments scaled per body axis by `scale`.
    pub(super) fn scaled_inv_world_tensor(&self, scale: Vec3) -> Mat3 {
        let inv = Mat3::from_diagonal(inverse_diagonal(self.inertia * scale));
        self.rotation * inv * self.rotation.transpose()
    }
}
