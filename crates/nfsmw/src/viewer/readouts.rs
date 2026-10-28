//! What a scene reports about the player's car, the camera and the world.

/// The player's car at one moment, in render space (x forward, y left, z up; metres and seconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCar {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    /// Heading, radians from +x towards +y.
    pub heading: f32,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    pub rpm: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// Nitrous left, 0 to 1; 0 without the part.
    pub nos: f32,
    /// Damage, 0 to 1 of the zones' levels.
    pub damage: f32,
    /// Body slip angle, radians, positive when the car moves to the left of its heading.
    pub slip_angle: f32,
    /// Steering, -1 to 1, positive to the left.
    pub steering: f32,
    /// The strongest hit of the latest physics step against the world or an object, 0 to 1.
    pub impact: f32,
    /// At least one wheel touches the ground.
    pub on_ground: bool,
}

/// The camera the player looks through, and the gameplay view it is in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraReadout {
    pub mode: crate::settings::CameraMode,
    pub position: [f32; 3],
    /// Unit length.
    pub forward: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
}

/// The track and the district the scene is in.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldInfo {
    pub track: String,
    /// The district at the player, `None` where the install names none.
    pub section: Option<String>,
}
