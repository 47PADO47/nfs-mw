//! The rider's placement and body dynamics (`docs/specs/ride6-rider-body-aero.md`): where his centre of mass is asked to sit
//! as a function of the weight inputs, the lean and the speed, how his body is held to the bike, and how much air he adds.
//! The mass, the rest place and the longitudinal shifts are in [`ChassisSpec`](super::ChassisSpec); everything here is
//! optional and the default is a rider who sits where the chassis puts him and never moves on his own.

use glam::Vec3;

/// The rider's placement and body dynamics. Offsets are in metres in the physics space (x right, y up, z forward).
#[derive(Clone, Debug, PartialEq)]
pub struct RiderSpec {
    /// Added to the rest place whatever the inputs are.
    pub base_offset: Vec3,
    /// The shift at full lean (or full lateral weight), moving: x toward the side of the lean (signed with it), y and z for
    /// either side (the size of the lean).
    pub lateral_offset: Vec3,
    /// The shifts that replace the moving ones at a standstill: longitudinal forward, backward, and lateral.
    pub stopped_forward_offset: Vec3,
    pub stopped_backward_offset: Vec3,
    pub stopped_lateral_offset: Vec3,
    /// The speeds (m/s) between which the stopped shifts give way to the moving ones: all stopped below the first, all moving
    /// above the second. A range of zero width is a step at its start, so the default (0, 0) never uses the stopped shifts.
    pub stopped_speed_range: [f32; 2],
    /// How much the lateral weight input and the normalised target lean each ask for the lateral shift (their sum is limited
    /// to -1 to 1). The weight input counts fully and the lean not at all unless the data says otherwise.
    pub weight_influence: f32,
    pub lean_influence: f32,
    /// The damper of the rider's translation is scaled from `damp_multiplier` at the lower speed (m/s) to 1 at the upper.
    pub damp_speed_range: [f32; 2],
    pub damp_multiplier: f32,
    /// Whether the spring rest place is raised so the rider does not sag under his own weight.
    pub correct_gravity: bool,
    /// The rider's moments of inertia as a multiple of his mass (m^2; x pitch, y yaw, z roll like the chassis), the stiffness
    /// (N m/rad) and damping (N m s/rad) that hold his body to the bike's attitude, and the share of the bike's angular
    /// velocity the damper works against (0: the rider's damping is against the world, 1: against the bike). A stiffness of 0
    /// leaves him rigid on the frame.
    pub inertia_scale: Vec3,
    pub angular_k: f32,
    pub angular_damp: f32,
    pub angular_damp_chassis_factor: f32,
    /// The per-axis scale of the reaction torque the rider's body puts on the bike (x pitch, y yaw, z roll).
    pub torque_scale: Vec3,
    /// The air drag the rider adds: a multiplier of the drag and its curve over the tuck (0 upright to 1 tucked in). An empty
    /// curve leaves the drag alone.
    pub drag_scale: f32,
    pub drag_curve: Vec<(f32, f32)>,
}

impl Default for RiderSpec {
    fn default() -> Self {
        Self {
            base_offset: Vec3::ZERO,
            lateral_offset: Vec3::ZERO,
            stopped_forward_offset: Vec3::ZERO,
            stopped_backward_offset: Vec3::ZERO,
            stopped_lateral_offset: Vec3::ZERO,
            stopped_speed_range: [0.0, 0.0],
            weight_influence: 1.0,
            lean_influence: 0.0,
            damp_speed_range: [40.0 / 3.6, 60.0 / 3.6],
            damp_multiplier: 1.0,
            correct_gravity: false,
            inertia_scale: Vec3::ONE,
            angular_k: 0.0,
            angular_damp: 0.0,
            angular_damp_chassis_factor: 1.0,
            torque_scale: Vec3::ONE,
            drag_scale: 1.0,
            drag_curve: Vec::new(),
        }
    }
}
