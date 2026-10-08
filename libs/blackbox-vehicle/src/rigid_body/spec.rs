use glam::Vec3;

/// Rigid-body parameters (the attribute class `rigidbodyspecs` plus the vehicle's mass data).
#[derive(Clone, Copy, Debug)]
pub struct RigidBodySpec {
    /// Gravity in m/s^2, signed: about -9.81 for a normal world. Added as `gravity * mass` to the y force.
    pub gravity: f32,
    /// Centre of gravity offset from the body origin, in the body frame (m).
    pub cg: Vec3,
    /// Quadratic linear drag coefficients per body axis (unitless). A zero vector disables the term.
    pub drag: Vec3,
    /// Quadratic angular drag coefficients per body axis (unitless). A zero vector disables the term.
    pub drag_angular: Vec3,
    /// Sleep threshold in m/s, compared with `|v| + |w| * radius`.
    pub sleep_velocity: f32,
    /// Added to the half extents of the ground-contact box (m).
    pub collision_box_pad: Vec3,
    /// Ground friction coefficients `[static, kinetic]`.
    pub ground_friction: [f32; 2],
    /// Ground restitution per body axis (right, up, forward).
    pub ground_elasticity: Vec3,
    /// Inertia multiplier per body axis in ground hits (1 = unchanged).
    pub ground_moment_scale: Vec3,
}

impl Default for RigidBodySpec {
    fn default() -> Self {
        Self {
            gravity: -9.81,
            cg: Vec3::ZERO,
            drag: Vec3::ZERO,
            drag_angular: Vec3::ZERO,
            sleep_velocity: 0.1,
            collision_box_pad: Vec3::ZERO,
            ground_friction: [0.8, 0.6],
            ground_elasticity: Vec3::splat(0.2),
            ground_moment_scale: Vec3::ONE,
        }
    }
}
