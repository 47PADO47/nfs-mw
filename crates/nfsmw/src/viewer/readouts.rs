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

/// Builds the tyre model of a mod for one car (`bb.handling`). The scene calls it for each car it puts the model on.
pub type TyreMaker = dyn Fn() -> Box<dyn blackbox_vehicle::tires::TyreModel>;
/// Builds the suspension model of a mod for one car (`bb.handling` 1.1).
pub type SuspensionMaker = dyn Fn() -> Box<dyn blackbox_vehicle::suspension::SuspensionModel>;
/// Builds the controls model of a mod for one car (`bb.handling` 1.2).
pub type ControlsMaker = dyn Fn() -> Box<dyn blackbox_vehicle::input::ControlsModel>;

/// The model builders of the hooks a mod has in place, for one frame. A kind with no builder has no live hook: the scene takes
/// that model off its car. A kind with a builder gets the model put on a car that lacks it.
#[derive(Clone, Copy, Default)]
pub struct HandlingMakers<'a> {
    pub tyre: Option<&'a TyreMaker>,
    pub suspension: Option<&'a SuspensionMaker>,
    pub controls: Option<&'a ControlsMaker>,
}

impl<'a> HandlingMakers<'a> {
    /// The builders of the hooks that are in place, as closures (a kind with `None` has no live hook).
    pub fn of<T, S, C>(tyre: Option<&'a T>, suspension: Option<&'a S>, controls: Option<&'a C>) -> Self
    where
        T: Fn() -> Box<dyn blackbox_vehicle::tires::TyreModel> + 'static,
        S: Fn() -> Box<dyn blackbox_vehicle::suspension::SuspensionModel> + 'static,
        C: Fn() -> Box<dyn blackbox_vehicle::input::ControlsModel> + 'static,
    {
        HandlingMakers {
            tyre: tyre.map(|make| make as &TyreMaker),
            suspension: suspension.map(|make| make as &SuspensionMaker),
            controls: controls.map(|make| make as &ControlsMaker),
        }
    }
}

/// What a scene reports for `bb.handling` (`Scene::handling_feed`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HandlingFeed {
    /// The scene is part of a networked session (a multiplayer client): hooks on the physics are refused.
    pub networked: bool,
    /// The physical summary of the player's car; `None` where no car is driven.
    pub summary: Option<blackbox_vehicle::VehicleSummary>,
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
