//! The camera data of one car: what `read_cameras` returns. Layouts and evidence: `docs/formats/chameleon-camera.md`.
//!
//! **Axes.** Every vector in these types is in the *host* space: right-handed, **x forward, y left, z up**, metres
//! (the space of the mod ABI's `BbVec3`). The 2012 files use x left, y up, z forward; the reader converts once
//! (`from_data_axes`), so nobody downstream has to. Angles are degrees, times are seconds, speeds are the file's:
//! **miles per hour** where a field is a speed.
//!
//! Field names below are not the file's (those are hashed, see the doc); they say what the number does as far as
//! the values show. Everything not backed by the executable is **[unconfirmed]** in the format doc.

/// A vector in host space: `[forward, left, up]` in metres (or a per-axis triple in that order).
pub type Vec3 = [f32; 3];

/// A 2012 data vector (x left, y up, z forward) as a host vector (x forward, y left, z up). A cyclic permutation of
/// the axes, so a proper rotation: handedness is kept.
pub const fn from_data_axes(v: Vec3) -> Vec3 {
    [v[2], v[0], v[1]]
}

/// The cameras of one car (`VehicleCameraContainer`).
#[derive(Debug, Clone, PartialEq)]
pub struct CarCameras {
    pub external: ExternalCamera,
    pub globals: ExternalGlobals,
    pub gradient: GradientAdjustments,
    pub bonnet: MountedCamera,
    pub bumper: MountedCamera,
    pub rear_view: RearView,
    /// The `Id` the container's data carries for itself (the key of the object that holds it in the bundle).
    pub object_key: u32,
}

/// The chase ("gameplay external") camera of one car. Per-car values: `focus`, `ground_pivot`, `position`, the
/// speed movement, the height change, the roll, the deceleration pitch and `lag_s`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalCamera {
    /// Per-axis response `[forward, left, up]` (1.0, 0.55, 0.09 in all 93 cars). **[unconfirmed]** role.
    pub response: Vec3,
    /// The point the camera looks at, relative to the car (host axes): about 0.75 m ahead of the origin and 1.2 m up.
    pub focus: Vec3,
    /// A second point with the same forward distance as `focus` and zero height (the focus projected on the
    /// ground). **[unconfirmed]** role; the model does not use it.
    pub ground_pivot: Vec3,
    /// The camera's rest position relative to the car: 4.6 m behind (x = -4.6) and 1.6 m up for most cars.
    pub position: Vec3,
    /// The two field-of-view numbers (95 and 70 in all cars): degrees, **horizontal at a 16:9 screen**
    /// **[unconfirmed]**; `fov_range_deg` sorts them.
    pub fov_deg: [f32; 2],
    /// 10 (5 for the SWAT van). **[unconfirmed]** role.
    pub param_168: f32,
    /// 4 in all cars. **[unconfirmed]** role.
    pub param_16c: f32,
    /// 0.045 to 0.07 s. **[unconfirmed]**: read as a short position lag; the model does not use it.
    pub lag_s: f32,
    pub speed_movement: SpeedMovement,
    pub accel_movement: AccelMovement,
    pub roll: CameraRoll,
    pub yaw_spring: YawSpring,
    pub decel_pitch: DecelPitch,
    /// `Speed_based_height_change`: 1.35 to 1.7. **[unconfirmed]**: the camera height at full speed in metres.
    pub height_at_speed: f32,
}

impl ExternalCamera {
    /// The smaller and the larger field-of-view number, in degrees.
    pub fn fov_range_deg(&self) -> [f32; 2] {
        [self.fov_deg[0].min(self.fov_deg[1]), self.fov_deg[0].max(self.fov_deg[1])]
    }
}

/// `Speed_based_movement`: what the camera does as the car speeds up.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeedMovement {
    /// -1.0 in 84 cars (-0.6 to 0). **[unconfirmed]**: metres the camera moves along the car's forward axis at full
    /// speed (negative = further back).
    pub back_at_speed: f32,
    /// 1.35 to 1.8. **[unconfirmed]**: a camera height; sits near `position[2]`. The model does not use it.
    pub height_a: f32,
    /// 1.5 to 3.0. **[unconfirmed]**: a camera height; sits near `position[2]`. The model does not use it.
    pub height_b: f32,
    /// 0 in all cars.
    pub param_40: f32,
    pub shake: HighSpeedShake,
}

/// `High_Speed_Shake`.
#[derive(Debug, Clone, PartialEq)]
pub struct HighSpeedShake {
    /// 1.2 (1.3 in ten cars). **[unconfirmed]**: strength of the shake at full speed.
    pub amplitude: f32,
    /// 1.0 in all cars. **[unconfirmed]**: a rate multiplier.
    pub rate: f32,
    /// 125 to 208, in mph. **[unconfirmed]**: the speed where the speed effects reach full (the Tesla Roadster
    /// has 125, its top speed). The model also uses it as the speed scale of the other effects.
    pub full_speed_mph: f32,
    /// 0 in all cars. **[unconfirmed]**: the speed the shake starts at.
    pub start_speed_mph: f32,
}

/// `Acceleration_movement`: ten numbers; the roles below are **[unconfirmed]** readings.
#[derive(Debug, Clone, PartialEq)]
pub struct AccelMovement {
    /// -2 (-1.9, -2.5): degrees of pitch at full acceleration.
    pub pitch_deg: f32,
    /// 0.5 s: smoothing time of the movement.
    pub smoothing_s: f32,
    /// 2.5. Role unknown.
    pub param_18: f32,
    /// 0. Role unknown.
    pub param_1c: f32,
    /// 50: the acceleration (mph per second) that counts as full.
    pub full_accel: f32,
    /// 1.5 m: how far the camera drops back at full acceleration.
    pub back_m: f32,
    /// -1.2 m: how far it moves forward at full braking (its magnitude is used).
    pub forward_m: f32,
    /// 2. Role unknown.
    pub param_2c: f32,
    /// 0 in 59 cars, -0.9 in 34. Role unknown.
    pub param_30: f32,
    /// 1.75. Role unknown.
    pub param_34: f32,
}

/// `Camera_Roll`: six numbers; roles **[unconfirmed]**.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraRoll {
    /// 2.5. Role unknown.
    pub param_10: f32,
    /// 2.5 (2.6, 3.0). Role unknown.
    pub param_14: f32,
    /// 30 (35, 40), in mph: the speed where the roll is fully on.
    pub full_speed_mph: f32,
    /// 4.7 (3.0, 5.6): the largest roll in degrees at full steering.
    pub max_deg: f32,
    /// 0.2 s: smoothing time.
    pub smoothing_s: f32,
    /// 10. Role unknown.
    pub param_24: f32,
}

/// `Yaw_spring_modification`: six numbers, identical in all 93 cars; roles **[unconfirmed]**.
#[derive(Debug, Clone, PartialEq)]
pub struct YawSpring {
    /// 0.01 m/s: the speed the spring starts to change at.
    pub speed_min: f32,
    /// 10: the spring's rate (rad/s) at `speed_min`.
    pub rate_low: f32,
    /// 100 m/s: the speed where the change ends.
    pub speed_max: f32,
    /// 0.05 s: how far ahead the camera aims along the car's yaw rate.
    pub lead_s: f32,
    /// 10: the spring's rate at `speed_max`.
    pub rate_high: f32,
    /// 0. Role unknown.
    pub param_24: f32,
}

/// `Deceleration_Pitch_Change`: four numbers; roles **[unconfirmed]**.
#[derive(Debug, Clone, PartialEq)]
pub struct DecelPitch {
    /// 12 (11): the fastest the pitch changes, degrees per second.
    pub rate_deg_s: f32,
    /// 4.5 (5.0, 2.5): pitch in degrees at full braking.
    pub max_deg: f32,
    /// 2. Role unknown.
    pub param_18: f32,
    /// 2.25: how fast the pitch follows its target (1/s).
    pub response: f32,
}

/// `CameraGameplayExternalGlobals`: shared by the cars (identical in all 93).
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalGlobals {
    pub impact_shake: ImpactShake,
    pub lens: Lens,
}

/// `Impact_shake`: the shake after a collision. Triples are per rotation axis in the **file's** order
/// `[pitch, yaw, roll]` (x, y, z of the data), not host axes. Roles **[unconfirmed]**.
#[derive(Debug, Clone, PartialEq)]
pub struct ImpactShake {
    /// (10, 10, 50): angular frequency in rad/s.
    pub frequency: [f32; 3],
    /// (1, 1, 1): amplitude in degrees at full strength.
    pub amplitude_deg: [f32; 3],
    /// (0.5, 0.5, 0.5): decay time constant in seconds.
    pub decay_s: [f32; 3],
    /// Nine more numbers from +0x34: 0.75, 0.165, 1, 60, 2, 30, 4, 5, 1. The first is read as the duration in
    /// seconds.
    pub scalars: [f32; 9],
}

/// `LensProperties`.
#[derive(Debug, Clone, PartialEq)]
pub struct Lens {
    /// 90. **[unconfirmed]**: a field of view in degrees (the largest the lens allows?); the model does not use it.
    pub fov_deg: f32,
    /// 1.4. **[unconfirmed]** role; the model does not use it.
    pub scale: f32,
}

/// `CameraGameplayGradientAdjustments`: two `Params`, 0.5 each in all cars. **[unconfirmed]**: the share of the
/// car's pitch the chase camera takes on when the car climbs (`[0]`) and descends (`[1]`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientAdjustments {
    pub pitch_follow: [f32; 2],
}

/// The bonnet and the bumper camera: one structure (`CameraGameplayBonnet`, `CameraGameplayBumper`).
#[derive(Debug, Clone, PartialEq)]
pub struct MountedCamera {
    /// (1.0, 0.5, 0.7) in all cars, as `[forward, left, up]`. **[unconfirmed]** role; unused by the model.
    pub response: Vec3,
    /// The camera position relative to the car. 92 of 93 cars share the bonnet's (1.318 up, 0.208 ahead) and 63
    /// share the bumper's: they are **not** per-car geometry **[unconfirmed]**, so a host that has the car's own
    /// anchor should add it (`MountedRig::new` takes one).
    pub eye: Vec3,
    /// The point it looks at, relative to the car.
    pub target: Vec3,
    /// +0x34: 0.3 s, orientation smoothing **[unconfirmed]**; unused by the model.
    pub smoothing_s: f32,
    /// +0x38: 0.05 m, sway under acceleration **[unconfirmed]**.
    pub sway_m: f32,
    /// +0x3C: 0.25, strength of the wind buffeting at full speed **[unconfirmed]**.
    pub wind_shake: f32,
    /// The two field-of-view numbers (bonnet 105 and 95, bumper 120 and 95): degrees, horizontal at 16:9
    /// **[unconfirmed]**.
    pub fov_deg: [f32; 2],
    /// +0x58 and +0x5C: 15 and 15 on the bonnet, 0 and 0 on the bumper. **[unconfirmed]**: look-around limits.
    pub look_limit_deg: [f32; 2],
    /// The other floats, by offset: +0x40, +0x44, +0x48, +0x4C, +0x50, +0x54, +0x60, +0x6C. Roles unknown.
    pub other: [f32; 8],
    /// The `Id` the structure carries (at +0x30).
    pub object_key: u32,
}

impl MountedCamera {
    /// The smaller and the larger field-of-view number, in degrees.
    pub fn fov_range_deg(&self) -> [f32; 2] {
        [self.fov_deg[0].min(self.fov_deg[1]), self.fov_deg[0].max(self.fov_deg[1])]
    }
}

/// `CameraRearView` and its globals: the camera of the rear-view mirror. **[unconfirmed]** reading: 3.3333 is the
/// mirror's aspect ratio (width over height), 400 its width in pixels and 60 its field of view.
#[derive(Debug, Clone, PartialEq)]
pub struct RearView {
    /// (0, 0.9, 0) in the data (x left, y up, z forward), so `[0, 0, 0.9]` here **[unconfirmed]**: the camera's
    /// offset; the model reads `up` as a height above the car's rear end.
    pub offset: Vec3,
    /// +0x1C: 0 in all cars.
    pub param_1c: f32,
    /// 3.3333.
    pub aspect: f32,
    /// 400.
    pub width_px: f32,
    /// 60.
    pub fov_deg: f32,
}
