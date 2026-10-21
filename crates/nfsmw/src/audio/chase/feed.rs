//! What the chase speech reads of the world besides the music's numbers: the cops (sight, distance, formation, health),
//! the helicopter and the roadblock, the player's heading, road and car, and how the pursuit stands. The world fills it
//! in `scenes/world/ai/police/speech_feed.rs` and it rides in [`PursuitFeed::speech`](crate::audio::PursuitFeed).
//! Everything defaults to "nothing known", so a feed built without it (the music tests) keeps working.

use blackbox_pursuit::InfractionLog;

pub use super::events::EventLog;

/// The most cops the feed carries (the speech has six voices besides dispatch and the helicopter).
pub const MAX_COPS: usize = 12;

/// What kind of car a cop drives, for the battalion it calls itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Vehicle {
    #[default]
    Patrol,
    /// An SUV (the Rhino units).
    Suv,
    /// A sports car of the super pursuit.
    SuperPursuit,
    /// The cross car, the one that speaks with its own voice.
    Cross,
}

/// One cop car.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CopFeed {
    /// A stable identity of the car.
    pub id: u32,
    /// The car is live; a wrecked car is listed for a while as inactive and destroyed.
    pub active: bool,
    pub destroyed: bool,
    /// Health, 1 whole to 0 wrecked.
    pub health: f32,
    /// Metres to the player.
    pub distance: f32,
    pub speed_mph: f32,
    /// Physics-space position.
    pub position: [f32; 3],
    /// The cop sees the player.
    pub los: bool,
    /// The cop is in the camera's view (approximated from sight and distance: the world has no camera test).
    pub in_view: bool,
    /// The cop is ahead of the player.
    pub ahead: bool,
    pub in_formation: bool,
    pub in_position: bool,
    /// The cop's offset to its slot, x right and z ahead of the player.
    pub offset: [f32; 2],
    pub vehicle: Vehicle,
    /// The speech id of the road the cop is on (0 untagged).
    pub road: u16,
    pub roadblock: bool,
}

/// The police helicopter.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HeliFeed {
    pub id: u32,
    /// It flies (not leaving).
    pub active: bool,
    pub los: bool,
    pub distance: f32,
    pub fuel_secs: f32,
}

/// A roadblock of the chase.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoadblockFeed {
    /// Stable while the roadblock stands.
    pub id: u32,
    pub visual: bool,
    pub spike_strips: u8,
    pub distance: f32,
}

/// How the pursuit stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Formation {
    #[default]
    Follow,
    StaggerFollow,
    Pit,
    BoxIn,
    RollingBlock,
    Herd,
}

/// The player's car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerFeed {
    /// Physics-space position, velocity direction (unit, zero when still) and forward.
    pub position: [f32; 3],
    pub velocity_dir: [f32; 3],
    pub forward: [f32; 3],
    /// The speech id of the road under the car (0 untagged); `None` when the car is off every road.
    pub road: Option<u16>,
    /// The speech class numbers of the car's type and colour (0 when the game cannot say); see `radio::tables`.
    pub car_type: u32,
    pub car_colour: u32,
    /// The direction the road runs at the car, held to the way the car first went (so it stays put through a U-turn);
    /// `None` off every road.
    pub road_forward: Option<[f32; 3]>,
    /// The brake and handbrake pedals, 0 to 1.
    pub brake: f32,
    pub handbrake: f32,
    /// The y component of the car's up vector: negative on its roof.
    pub up_y: f32,
    /// The id of the place under the car on the speech map (an off-road moment), when the track has one.
    pub offroad: Option<u8>,
}

impl Default for PlayerFeed {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            velocity_dir: [0.0; 3],
            forward: [0.0, 0.0, 1.0],
            road: None,
            car_type: 0,
            car_colour: 0,
            road_forward: None,
            brake: 0.0,
            handbrake: 0.0,
            up_y: 1.0,
            offroad: None,
        }
    }
}

/// The state of the pursuit the speech machine reads.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StatusFeed {
    pub cooling_down: bool,
    pub formation: Formation,
    pub collapse_active: bool,
    pub finisher_active: bool,
    /// The escape meter, 0 to 1.
    pub evade_level: f32,
    /// The busted meter, 0 to 1.
    pub busted: f32,
    /// The infractions committed in this chase.
    pub infractions: InfractionLog,
}

/// Everything the chase speech reads.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpeechFeed {
    pub cops: [CopFeed; MAX_COPS],
    pub cop_count: u8,
    pub heli: Option<HeliFeed>,
    pub roadblock: Option<RoadblockFeed>,
    pub player: PlayerFeed,
    pub status: StatusFeed,
    /// What happened in the world lately that the cops remark on.
    pub events: EventLog,
}

impl SpeechFeed {
    /// The cops that are listed.
    pub fn cops(&self) -> &[CopFeed] {
        &self.cops[..usize::from(self.cop_count).min(MAX_COPS)]
    }

    /// Lists a cop (nothing happens past [`MAX_COPS`]).
    pub fn push_cop(&mut self, cop: CopFeed) {
        let at = usize::from(self.cop_count);
        if at >= MAX_COPS {
            return;
        }
        self.cops[at] = cop;
        self.cop_count += 1;
    }
}
