//! What the machine reads each step: a [`Snapshot`] of the pursuit and the cars, the [`Notice`]s (the game's messages and
//! collisions since the last step) and the [`Channel`] (what the radio queue is doing). What it gives back is in
//! [`Output`](super::Output).

use glam::Vec3;

use super::line::{Line, Speaker};
use super::params::{Backup, Battalion, Car, Params};
use crate::infraction::Infraction;

/// The strategy a pursuit's cops follow (the formation type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    Pit,
    BoxIn,
    RollingBlock,
    Follow,
    StaggerFollow,
    Herd,
    HeliPursuit,
}

/// How a pursuit stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PursuitInfo {
    /// The player has a pursuit.
    pub exists: bool,
    /// Only an AI racer is pursued.
    pub other_target: bool,
    pub re_acquiring: bool,
    pub perp_in_sight: bool,
    /// The status is cool-down, with `cooldown_remaining` seconds to run.
    pub cooling_down: bool,
    pub cooldown_remaining: f32,
    pub evaded: bool,
    pub strategy: Strategy,
    pub collapse_active: bool,
    pub finisher_active: bool,
    /// 0 to 1, how far the escape has come.
    pub evade_level: f32,
    pub backup_eta: f32,
    /// The busted meter, 0 to 1.
    pub busted: f32,
    pub cops_destroyed: u32,
    /// How many pursuits the player has had in this speech session (an opening for each).
    pub pursuits: u32,
    pub heat: f32,
}

impl Default for PursuitInfo {
    fn default() -> Self {
        Self {
            exists: false,
            other_target: false,
            re_acquiring: false,
            perp_in_sight: false,
            cooling_down: false,
            cooldown_remaining: 0.0,
            evaded: false,
            strategy: Strategy::Follow,
            collapse_active: false,
            finisher_active: false,
            evade_level: 0.0,
            backup_eta: 0.0,
            busted: 0.0,
            cops_destroyed: 0,
            pursuits: 1,
            heat: 1.0,
        }
    }
}

/// The player's car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerInfo {
    pub speed_mph: f32,
    pub position: Vec3,
    /// The direction of the velocity, unit length (zero when it is not moving).
    pub velocity_dir: Vec3,
    pub forward: Vec3,
    /// The speech id of the road under the car (0 untagged); `None` when it cannot be told.
    pub road: Option<u16>,
    /// The forward vector of the road at the car, when there is one.
    pub road_forward: Option<Vec3>,
    pub brake: f32,
    pub handbrake: f32,
    /// The y component of the car's up vector (negative when it is upside down).
    pub up_y: f32,
    /// The car's type and colour for descriptions, `None` when the game cannot say.
    pub car: Option<Car>,
    /// A car the cops may describe (not a cop car, not a traffic car).
    pub commentable: bool,
    /// The speedometer shows kilometres per hour.
    pub metric: bool,
    /// The speech map id under the car (an off-road place), when there is one.
    pub offroad_id: Option<u8>,
}

impl Default for PlayerInfo {
    fn default() -> Self {
        Self {
            speed_mph: 0.0,
            position: Vec3::ZERO,
            velocity_dir: Vec3::ZERO,
            forward: Vec3::Z,
            road: None,
            road_forward: None,
            brake: 0.0,
            handbrake: 0.0,
            up_y: 1.0,
            car: None,
            commentable: true,
            metric: false,
            offroad_id: None,
        }
    }
}

/// One cop car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopInfo {
    /// A stable identity for the car.
    pub id: u32,
    pub active: bool,
    pub in_view: bool,
    /// The cop sees the player (sighted within the last frames, or a roadblock car in view).
    pub los: bool,
    pub ahead: bool,
    pub distance: f32,
    pub speed_mph: f32,
    pub position: Vec3,
    pub health: f32,
    pub destroyed: bool,
    pub in_formation: bool,
    pub in_position: bool,
    /// The cop's offset to the target (x right, z ahead).
    pub offset: (f32, f32),
    /// The battalion this car's model implies (an SUV, a super pursuit car).
    pub vehicle_battalion: Option<Battalion>,
    /// The battalion of the area its road is in.
    pub road_battalion: Option<Battalion>,
    pub cross: bool,
    pub roadblock: bool,
}

impl Default for CopInfo {
    fn default() -> Self {
        Self {
            id: 0,
            active: true,
            in_view: false,
            los: false,
            ahead: false,
            distance: 100.0,
            speed_mph: 0.0,
            position: Vec3::ZERO,
            health: 1.0,
            destroyed: false,
            in_formation: false,
            in_position: false,
            offset: (0.0, 0.0),
            vehicle_battalion: None,
            road_battalion: None,
            cross: false,
            roadblock: false,
        }
    }
}

/// The police helicopter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeliInfo {
    pub id: u32,
    pub active: bool,
    pub los: bool,
    pub distance: f32,
    pub health: f32,
    /// Seconds of fuel left, when the helicopter burns any.
    pub fuel_secs: Option<f32>,
}

/// A roadblock of the pursuit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoadblockInfo {
    /// Changes when a new roadblock replaces the old.
    pub id: u32,
    /// Some car of it sees the player or is in view.
    pub visual: bool,
    pub spike_strips: u8,
    pub cops_damaged: u32,
    pub cops_destroyed: u32,
    pub distance: f32,
}

/// The settings that gate the roadblock chatter, and the free-roam 911.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Level {
    pub roadblocks: bool,
    pub heli_roadblocks: bool,
    pub spikes: bool,
    /// Free roam: the cost to state that makes the dispatcher call 911 (0 or less: never).
    pub cts_for_911: i32,
    pub hits_for_911: i32,
    /// Seconds the 911 call keeps the cops away.
    pub lifetime_911: f32,
    pub cops_enabled: bool,
    pub roaming: bool,
    pub ddays: bool,
    pub pursuit_race: bool,
    /// Seconds left of the cop lockout (0 when none).
    pub lockout_remaining: f32,
}

/// Everything the machine reads of the game in one step.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub pursuit: PursuitInfo,
    pub player: PlayerInfo,
    pub cops: Vec<CopInfo>,
    pub heli: Option<HeliInfo>,
    pub roadblock: Option<RoadblockInfo>,
    pub level: Level,
    /// AI racers are within hearing of the pursuit.
    pub racers_near: bool,
    /// Rain is falling or the road is wet (changes the bail-out wording).
    pub weather: bool,
}

/// How the player's car met a cop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Impact {
    RearEnd,
    TBone,
    HeadOn,
    SideSwipe,
    /// A ram the original has no wording for (both noses in): nothing is said at normal intensity.
    Other,
    /// A hit that is no ram (the cop hit the player).
    None,
}

/// What a roadblock was hit by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoadblockHit {
    Spikes,
    Object,
    Vehicle,
}

/// A message or collision since the last step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Notice {
    /// The player was busted: everything stops.
    Busted,
    /// An AI racer was busted.
    AiBusted,
    /// The race was restarted.
    Restart,
    /// The infraction message (the last one is the pursuit's wording).
    Infraction(Infraction),
    /// A cop was unspawned: removed outright, or only made inactive.
    Unspawn {
        cop: u32,
        remove: bool,
    },
    BackupRequested(Backup),
    BackupDenied(Backup),
    RoadblockDodged,
    /// The roadblock's cars were placed; the spike strip's offset from the road's centre.
    RoadblockPosition(i32),
    HeliJoinsRoadblock,
    TireBlown,
    RoadblockHit(RoadblockHit),
    /// The player passed a cop at a much higher speed.
    BlewByCop(u32),
    Tunnel(bool),
    GasStationExploded {
        by_player: bool,
        at: Vec3,
    },
    /// The player's car hit a cop. `deliberate` is false when the cop was braking, in a roadblock, an SUV or immune.
    PlayerHitCop {
        cop: u32,
        impact: Impact,
        intensity: f32,
        deliberate: bool,
    },
    CopHitTraffic {
        cop: u32,
    },
    /// The player hit the world; `crash` when it lost more speed than the tuning's crash slowdown.
    PlayerHitWorld {
        intensity: f32,
        crash: bool,
    },
    /// The player's car lost most of its speed against a car or a prop (the crash rule that `PlayerHitWorld` carries for
    /// the static world): it only marks the time of the last crash.
    Crashed,
    /// The player hit traffic; `semi` for a tractor or trailer.
    PlayerHitTraffic {
        intensity: f32,
        semi: bool,
    },
    PlayerHitSuspect,
    PlayerHitSpikeBelt {
        intensity: f32,
    },
    CopHitWorld {
        cop: u32,
        intensity: f32,
        visible: bool,
    },
    /// The player smashed property worth this much to the state.
    Smashed {
        cost: i32,
    },
    /// The player was in the air at this altitude for this long.
    Airborne {
        altitude: f32,
        secs: f32,
    },
    /// A line finished playing.
    Said {
        line: Line,
        speaker: Speaker,
        params: Params,
    },
    /// The game ends the pursuit for the cops' sake (forced bail-out).
    ForcedBailout,
}

/// What the radio queue is doing. The machine asks before it speaks.
pub trait Channel {
    /// A line is playing, held, or waiting in the queue.
    fn busy(&self) -> bool;
    /// Seconds since a line last sounded (0 while one does).
    fn dead_air(&self) -> f32;
    /// How often the line was said.
    fn said(&self, line: Line) -> u32;
    fn queued(&self, line: Line) -> bool;
    /// The line playing and its parameters.
    fn playing(&self) -> Option<(Line, Params)>;
    /// The line said last, and who said it.
    fn last(&self) -> Option<(Line, Speaker)>;

    fn is_playing(&self, line: Line) -> bool {
        self.playing().is_some_and(|(playing, _)| playing == line)
    }
}
