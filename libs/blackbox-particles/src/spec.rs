//! The parameters of one emitter.

use glam::Vec3;

/// What an emitter spawns and how its particles live. `variance` fields are fractions of the value they
/// belong to: a particle's value is drawn from `[value * (1 - variance), value)`.
#[derive(Debug, Clone, PartialEq)]
pub struct EmitterSpec {
    /// Particles per second at intensity 1.
    pub rate: f32,
    /// Fraction taken off `rate` (a fixed reduction, not a random one).
    pub rate_variance: f32,
    /// Seconds a particle lives. Also the scale of the curves' parameter: see [`crate::Emitter`].
    pub life: f32,
    pub life_variance: f32,
    /// Speed along the emitter's local `z` axis, in units per second. 0 uses the velocity ranges below.
    pub speed: f32,
    pub speed_variance: f32,
    /// Full angle in degrees of the cone the direction is spread over (each axis gets `+-spread / 2`).
    pub spread: f32,
    /// With `speed == 0`: start velocity and its random half-range (per local axis, or world axes when
    /// `world_axis_velocity`), start acceleration and its half-range (used only when `gravity` is 0).
    pub velocity_start: Vec3,
    pub velocity_delta: Vec3,
    pub accel_start: Vec3,
    pub accel_delta: Vec3,
    /// The velocity ranges are world-space vectors rather than along the emitter's axes.
    pub world_axis_velocity: bool,
    /// Where particles are born: a box of `volume_extent` centred on `volume_center`, in the emitter's frame.
    pub volume_center: Vec3,
    pub volume_extent: Vec3,
    /// Quadratic drag coefficient: `v += v * -dt * drag * |v|`.
    pub drag: f32,
    /// Subtracted from the vertical velocity each second (a negative value lifts). 0 means "use the
    /// acceleration" instead.
    pub gravity: f32,
    /// Share of the frame's inherited velocity a particle takes at birth, with its variance.
    pub inherit: f32,
    pub inherit_variance: f32,
    /// Particles do not take the inherited velocity at birth: they stay where they were born.
    pub live_motion: bool,
    /// Degrees; the starting rotation is drawn from `[-range / 2, range / 2)`.
    pub initial_angle_range: f32,
    /// Fraction (0..1) of the rotation curve a particle adds on top, drawn per particle.
    pub rotation_variance: f32,
    /// Half of the particles turn the other way.
    pub random_rotation_direction: bool,
    /// Where the four curve values sit along the particle's life, 0..1.
    pub keys: [f32; 4],
    /// Full width of the sprite at each key (the sprite's half size is half of it).
    pub size: [f32; 4],
    /// Rotation in degrees added to the initial angle at each key.
    pub angle: [f32; 4],
    /// RGBA at each key.
    pub colors: [[u8; 4]; 4],
    /// A particle whose alpha falls to this value or below dies (`None`: never).
    pub kill_alpha: Option<u8>,
}

impl Default for EmitterSpec {
    fn default() -> Self {
        Self {
            rate: 0.0,
            rate_variance: 0.0,
            life: 1.0,
            life_variance: 0.0,
            speed: 0.0,
            speed_variance: 0.0,
            spread: 0.0,
            velocity_start: Vec3::ZERO,
            velocity_delta: Vec3::ZERO,
            accel_start: Vec3::ZERO,
            accel_delta: Vec3::ZERO,
            world_axis_velocity: false,
            volume_center: Vec3::ZERO,
            volume_extent: Vec3::ZERO,
            drag: 0.0,
            gravity: 0.0,
            inherit: 0.0,
            inherit_variance: 0.0,
            live_motion: false,
            initial_angle_range: 0.0,
            rotation_variance: 0.0,
            random_rotation_direction: false,
            keys: [0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0],
            size: [1.0; 4],
            angle: [0.0; 4],
            colors: [[255; 4]; 4],
            kill_alpha: Some(0),
        }
    }
}
