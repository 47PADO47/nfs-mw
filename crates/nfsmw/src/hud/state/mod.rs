//! The HUD state: what the in-game HUD shows, as plain data the scene fills every frame. The original package's fields
//! sit on [`HudState`] itself; the data the Custom HUD adds is grouped in sub-structs:
//!
//! - `vehicle`: the extra telemetry and the car's condition;
//! - `race_info`: the race type, the next gate, the route left and the rival gap;
//! - `camera`: the gameplay camera, for world-space markers;
//! - `police`: the phase of the chase and what the player can know of the cops;
//! - `nav`: the GPS route with its next turn, and the places on the map;
//! - `events`: what changed this frame, once per change;
//! - `marks`: the places the world markers stand on, with their occlusion.

use bevy_ecs::prelude::Resource;
use blackbox_minimap::Orientation;

use super::blip_style::MapBlip;
use super::pursuit::PursuitHud;

mod camera;
mod events;
mod marks;
mod nav;
mod police;
mod race_info;
mod vehicle;

pub use camera::HudCamera;
pub use events::HudEvent;
pub use marks::{EntranceMark, MarkPoint, WorldMarks};
#[cfg(test)]
pub use nav::TurnDirection;
pub use nav::{NavMarker, NavMarkerKind, NavRoute, next_turn};
pub use police::{HelicopterIntel, LastKnown, PoliceIntel, PursuitPhase, known_roadblocks, sighted};
pub use race_info::{RaceInfo, RivalGap, checkpoint_gap, rival_of, shows_rival_gap};
pub use vehicle::{ConditionWarning, HEAVY_DAMAGE, HudTelemetry, TireCondition, VehicleCondition, cosmetic_level};

/// Where the car is, for the minimap (`docs/specs/hud-minimap.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPosition {
    /// The car's place in the world, metres (render space: x forward, y left).
    pub position: [f32; 2],
    /// The direction the car points in, in the same space (any length).
    pub heading: [f32; 2],
    /// North up (the fixed minimap) or the heading up (the rotating one).
    pub orientation: Orientation,
}

/// What the in-game HUD shows, as plain numbers. The scene fills it every frame; nothing here knows how
/// the HUD is built or drawn.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct HudState {
    pub vehicle_heat: f32,
    pub radar: Option<super::RadarHud>,
    pub cops_engaged: u32,
    pub cops_damaged: u32,
    /// Draw the HUD at all.
    pub visible: bool,
    /// Speed in metres per second.
    pub speed: f32,
    /// Show miles per hour instead of kilometres per hour.
    pub use_mph: bool,
    /// Engine speed in rpm.
    pub rpm: f32,
    /// The engine's `MAX_RPM`: it picks the tachometer face and the end of the needle's sweep.
    pub max_rpm: f32,
    /// The engine's red line: where the red zone of the tachometer starts.
    pub red_line: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// A gear change is in progress (the gear digit dims).
    pub shifting: bool,
    /// The shift light is on: the gearbox wants the next gear.
    pub shift_light: bool,
    /// Tachometer skin (the `HUDS_Custom_NN` pack, 0..=10).
    pub skin: u8,
    /// Backing, needle and digits. None keeps the authored color.
    pub gauge_colors: [Option<[u8; 3]>; 3],
    /// The car has a nitrous system: the nitrous gauge is shown.
    pub has_nos: bool,
    /// Nitrous left, 0..1.
    pub nos: f32,
    /// The speed breaker's meter, 0..1 (`scenes/world/drive/speedbreaker.rs`).
    pub speedbreaker: f32,
    /// Active Speedbreaker mode: drives the package fade and central meter.
    pub speedbreaker_active: bool,
    /// The car has forced induction: the turbo gauge is shown.
    pub has_turbo: bool,
    /// Boost gauge, psi (negative in vacuum).
    pub boost_psi: f32,
    /// The minimap shows this place; `None` hides it (a scene that has no map position).
    pub minimap: Option<MapPosition>,
    /// The second co-op player's car, for the world map's second mark (`scenes/world/hud.rs`); `None` outside a co-op
    /// session.
    pub second_player: Option<MapPosition>,
    /// The cops, racers, checkpoint and helicopter on the map (`docs/specs/hud-minimap.md` section 7).
    pub blips: Vec<MapBlip>,
    /// A police car is near: the radar's green laser shows over the base radar. The world sets it while a chase is on.
    pub cop_nearby: bool,
    /// The chase the pursuit board shows (its timer, the cool-down bar); `None` while no cop chases the player.
    pub pursuit: Option<PursuitHud>,
    /// The busted meter, 0 to 1: the share of the busted timer filled, 1 once the player is busted. Zero with no chase.
    pub busted: f32,
    /// The busted overlay (`BUSTED_OVERLAY.fng`) is up: the surveillance-camera look after the arrest shot. The world sets
    /// it while the bust sequence shows the overlay (`scenes/world/ai/police/busted/flow.rs`).
    pub busted_overlay: bool,
    /// Career mode is on: the bounty row's number, the car's banked bounty plus the chase's pending points. `None`
    /// outside career mode, where the bounty row and the heat meter stay hidden.
    pub career_bounty: Option<u32>,
    /// The race the player runs: the place, lap, clock and countdown. `None` outside a race.
    pub race: Option<super::race::RaceHud>,
    /// The leaderboard of the race (`HUD_SingleRace.fng`'s `LeaderBoardGroup`); `None` outside a race.
    pub leaders: Option<super::leaders::Leaders>,
    /// The race is a drag race: the HUD shows `HUD_Drag.fng` instead of `HUD_SingleRace.fng` (the layout number 2 of
    /// `docs/specs/race-mode.md` section 6).
    pub drag: bool,
    /// The line the generic message group shows (`hud/race.rs`).
    pub message: Option<super::race::Message>,
    /// Which map file the minimap shows: the stage's outside a race, the race's own tiles in one
    /// (`docs/specs/race-minimap.md`).
    pub map_source: super::map_source::MapSource,
    /// The extra telemetry of the car: the nitrous burn, the shift point, the engine load and the lateral acceleration.
    pub telemetry: HudTelemetry,
    /// The car's body, tyres and whether it still drives; `None` without a simulated car.
    pub condition: Option<VehicleCondition>,
    /// The race beyond the package's readout (the type, the next gate, the route left, the rival gap); `None` outside a
    /// race.
    pub race_info: Option<RaceInfo>,
    /// The camera the world is drawn from, for world-space markers; `None` in a scene without one.
    pub camera: Option<HudCamera>,
    /// The career's cash; `None` outside career mode.
    pub cash: Option<u32>,
    /// The phase of the chase (`None` with no chase and no end card up).
    pub pursuit_phase: PursuitPhase,
    /// What the player can know of the police: sightings, the last-known place, the helicopter, roadblocks.
    pub police_intel: PoliceIntel,
    /// The GPS route with its next turn; `None` while no route is set.
    pub route: Option<NavRoute>,
    /// The places on the map the player can go to (race starts, safe houses, shops), as the map shows them.
    pub markers: Vec<NavMarker>,
    /// What changed this frame, once per change (`scenes/world/feed/hud.rs`).
    pub events: Vec<HudEvent>,
    /// The frame `events` belong to: a reader that gets the same frame twice (a paused scene) does not take them again.
    pub events_frame: u64,
    /// The places the world markers stand on: the ground under the places near the car, the breakers in sight, and
    /// which of them the static world hides.
    pub world_marks: WorldMarks,
}

impl Default for HudState {
    fn default() -> Self {
        Self {
            vehicle_heat: 0.0,
            radar: None,
            cops_engaged: 0,
            cops_damaged: 0,
            visible: true,
            busted: 0.0,
            busted_overlay: false,
            career_bounty: None,
            race: None,
            leaders: None,
            drag: false,
            message: None,
            map_source: Default::default(),
            speed: 0.0,
            use_mph: false,
            rpm: 0.0,
            max_rpm: 8000.0,
            red_line: 7500.0,
            gear: 0,
            shifting: false,
            shift_light: false,
            skin: 0,
            gauge_colors: [None; 3],
            has_nos: false,
            nos: 0.0,
            speedbreaker: 0.0,
            speedbreaker_active: false,
            has_turbo: false,
            boost_psi: 0.0,
            minimap: None,
            second_player: None,
            blips: Vec::new(),
            cop_nearby: false,
            pursuit: None,
            telemetry: HudTelemetry::default(),
            condition: None,
            race_info: None,
            camera: None,
            cash: None,
            pursuit_phase: PursuitPhase::None,
            police_intel: PoliceIntel::default(),
            route: None,
            markers: Vec::new(),
            events: Vec::new(),
            events_frame: 0,
            world_marks: WorldMarks::default(),
        }
    }
}

impl HudState {
    /// The speed in the unit shown. The game shows whole units, cut off, not rounded (the digits are the integer
    /// parts of the speed), and three digits at most.
    pub fn display_speed(&self) -> u32 {
        let v = if self.use_mph { self.speed * 2.236_936 } else { self.speed * 3.6 };
        v.max(0.0).floor().min(999.0) as u32
    }

    /// The tachometer scale: the engine's `MAX_RPM` rounded up to the face the game has art for (7000 to
    /// 10000, in steps of 1000 and strictly above `MAX_RPM`).
    pub fn scale_rpm(&self) -> f32 {
        scale_rpm(self.max_rpm)
    }

    /// 0..1 along the tachometer scale.
    pub fn rpm_fraction(&self) -> f32 {
        (self.rpm / self.scale_rpm()).clamp(0.0, 1.0)
    }
}

/// The tachometer face for an engine: the first of 7000, 8000, 9000 that is above `max_rpm`, else 10000.
pub fn scale_rpm(max_rpm: f32) -> f32 {
    [7000.0, 8000.0, 9000.0].into_iter().find(|&n| max_rpm < n).unwrap_or(10000.0)
}

/// The angle (degrees) the redline mask of the tachometer is turned to: the bigger it is, the shorter the red zone.
/// The values are the game's table by `MAX_RPM` band and red line.
pub fn redline_rotation(max_rpm: f32, red_line: f32) -> f32 {
    // Per band: the red lines (rpm) from which each angle applies, highest first.
    let table: &[(f32, f32)] = match max_rpm {
        m if m < 7000.0 => &[(6500.0, 164.5), (6000.0, 149.5), (5500.0, 131.5), (0.0, 113.5)],
        m if m < 8000.0 => &[(7500.0, 165.0), (7000.0, 152.0), (6500.0, 138.0), (6000.0, 123.0), (0.0, 110.0)],
        m if m < 9000.0 => &[(8500.0, 166.0), (8000.0, 154.0), (7500.0, 140.5), (7000.0, 127.0), (0.0, 115.0)],
        _ => &[(9500.0, 167.0), (9000.0, 156.0), (8500.0, 145.0), (8000.0, 134.0), (0.0, 123.0)],
    };
    table.iter().find(|(from, _)| red_line >= *from).map_or(0.0, |(_, angle)| *angle)
}

/// The same mask angle for the drag tachometer, from the executable's own table (the drag branch of the tach face routine, which picks
/// it by the same `MAX_RPM` bands and red lines as the race HUD's, with these angles) **[exe]**.
pub fn redline_rotation_drag(max_rpm: f32, red_line: f32) -> f32 {
    let table: &[(f32, f32)] = match max_rpm {
        m if m < 7000.0 => &[(6500.0, 39.25), (6000.0, 32.25), (5500.0, 26.0), (0.0, 19.25)],
        m if m < 8000.0 => &[(7500.0, 40.0), (7000.0, 34.0), (6500.0, 28.25), (6000.0, 22.5), (0.0, 17.25)],
        m if m < 9000.0 => &[(8500.0, 42.0), (8000.0, 37.0), (7500.0, 32.25), (7000.0, 27.0), (0.0, 22.0)],
        _ => &[(9500.0, 41.5), (9000.0, 37.0), (8500.0, 31.5), (8000.0, 27.0), (0.0, 22.75)],
    };
    table.iter().find(|(from, _)| red_line >= *from).map_or(0.0, |(_, angle)| *angle)
}

#[cfg(test)]
mod tests;
