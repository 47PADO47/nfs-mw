//! The world-space places the Custom HUD's markers stand on, as the scene finds them (`scenes/world/hud_marks.rs`): the
//! ground under the map places near the car, the pursuit breakers in sight during a chase, and whether the static world
//! hides each of them, and the next gate, from the camera. The markers are drawn by the Custom theme
//! (`ui/theme/custom/markers`).

/// A place in the world a marker stands on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkPoint {
    /// The ground under the place, render space (x, y, z), metres.
    pub position: [f32; 3],
    /// The static world blocks the line from the camera to a point just above the place.
    pub occluded: bool,
}

/// A place of [`HudState::markers`](super::HudState::markers) near the car, with its ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntranceMark {
    /// The index of the place in `HudState::markers`.
    pub marker: usize,
    pub point: MarkPoint,
}

/// The places the world markers stand on this frame. Empty in a scene without a camera or a car.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldMarks {
    /// The static world hides the next gate (`RaceInfo::next_gate`) from the camera.
    pub gate_occluded: bool,
    /// The map places within the entrance range of the car, nearest first.
    pub entrances: Vec<EntranceMark>,
    /// The pursuit breakers within sight distance of the car while a chase is on, nearest first.
    pub breakers: Vec<MarkPoint>,
    /// The car's height, render z, metres: the height the markers give the places known only across the ground (the
    /// police intel). `None` without a car.
    pub car_z: Option<f32>,
}
