//! What every scenario starts from: the fixture car, the camera the world markers project through, and the base state.
//!
//! The car sits at a fixed place in the city, pointing along +x (render space: x forward, y left, z up), the same place
//! the screenshot scripts start the world at (`view-world --drive --at 2150,1399`). The fixture's ground is the plane
//! z = 0 and its camera looks over the car's roof along the heading; the world markers project through that camera, not
//! the scene's, so they land on the same pixels whatever the scene's camera does.

use blackbox_minimap::Orientation;
use glam::Vec3;

use crate::hud::{HudCamera, HudState, HudTelemetry, MapPosition, VehicleCondition};
use crate::viewer::camera::Lens;

/// The fixture car's place, render space (x, y), metres.
pub const CAR: [f32; 2] = [2150.0, 1399.0];
/// The way the car points: along +x.
pub const HEADING: [f32; 2] = [1.0, 0.0];
/// The window a state is built for when the window's own size is not known, logical pixels.
pub const VIEWPORT: [f32; 2] = [1280.0, 720.0];
/// The camera: behind the car and above its roof, looking down the road ahead.
const EYE_BEHIND: f32 = 7.0;
const EYE_HEIGHT: f32 = 2.4;
const LOOK_AHEAD: f32 = 20.0;
const LOOK_HEIGHT: f32 = 0.8;
const FOV_Y_DEGREES: f32 = 55.0;
const NEAR: f32 = 0.3;

/// A place `ahead` metres in front of the car and `left` metres to its left, render space (x, y).
pub fn at(ahead: f32, left: f32) -> [f32; 2] {
    [CAR[0] + ahead, CAR[1] + left]
}

/// The same place on the fixture's ground, render space (x, y, z).
pub fn on_ground(ahead: f32, left: f32) -> [f32; 3] {
    let [x, y] = at(ahead, left);
    [x, y, 0.0]
}

/// The fixture's camera for a window of `viewport` logical pixels.
pub fn camera(viewport: [f32; 2]) -> HudCamera {
    let eye = Vec3::new(CAR[0] - EYE_BEHIND, CAR[1], EYE_HEIGHT);
    let target = Vec3::new(CAR[0] + LOOK_AHEAD, CAR[1], LOOK_HEIGHT);
    let aspect = viewport[0] / viewport[1].max(1.0);
    let lens = Lens::look_at(eye, target, FOV_Y_DEGREES, NEAR, aspect);
    HudCamera { view_proj: lens.view_proj().to_cols_array_2d(), viewport }
}

/// The fixture car at `kmh` in `gear` at `rpm`: an 8000 rpm engine with nitrous half full and a turbo, the shift point
/// at 7200 rpm below the top gear, light body wear, on the map at its place, seen through the fixture's camera.
pub fn car(kmh: f32, gear: i32, rpm: f32, viewport: [f32; 2]) -> HudState {
    let red_line = 7500.0;
    HudState {
        speed: kmh / 3.6,
        rpm,
        max_rpm: 8000.0,
        red_line,
        gear,
        shift_light: rpm > red_line,
        has_nos: true,
        nos: 0.5,
        has_turbo: true,
        boost_psi: 9.5,
        minimap: Some(MapPosition { position: CAR, heading: HEADING, orientation: Orientation::North }),
        telemetry: HudTelemetry {
            nos_burning: false,
            shift_up_rpm: (1..6).contains(&gear).then_some(7200.0),
            throttle: 0.45,
            lateral_g: 0.12,
        },
        condition: Some(VehicleCondition { cosmetic: Some(0.05), ..VehicleCondition::default() }),
        camera: Some(camera(viewport)),
        ..HudState::default()
    }
}

/// The career numbers free roam and the chases show: the cash and the bounty.
pub fn career(state: HudState) -> HudState {
    HudState { cash: Some(48_250), career_bounty: Some(125_000), ..state }
}
