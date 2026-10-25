//! The HUD's events: what changed in the game this frame, once per change. The world detects them
//! (`scenes/world/feed/`) and the HUD state carries the frame's list; the notification manager turns them into notices
//! (`hud/notify/`). An identical state over many frames makes no event.

use super::PursuitPhase;

/// One change the HUD can tell the player of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HudEvent {
    /// The chase moved from one phase to another.
    PhaseChanged { from: PursuitPhase, to: PursuitPhase },
    /// The chase's heat rose to `level`.
    HeatUp { level: u32 },
    /// Dispatch put a roadblock in the chase.
    RoadblockReported,
    /// The police helicopter got the player in sight.
    HelicopterTracking,
    /// The cops lost sight of the player: the search begins.
    ContactLost,
    /// The cool-down meter came up.
    CooldownStarted,
    /// The player got away.
    Escaped,
    /// The player was caught.
    Busted,
    /// The player passed checkpoint `number` of the lap's `total`.
    Checkpoint { number: u32, total: u32 },
    /// The player drove into the race's wrong-way zone: the race goes back to its grid.
    WrongWay,
    /// The player began lap `lap` of `laps`.
    Lap { lap: u32, laps: u32 },
    /// The player's place in the race changed (1 for first).
    PositionChanged { from: u32, to: u32 },
    /// The player passed a traffic car close by.
    NearMiss,
    /// The player drove through a speed trap at `kmh`; `passed` when it was fast enough to count.
    SpeedTrap { kmh: f32, passed: bool },
    /// The bounty passed a milestone: `bounty` is the milestone reached.
    BountyMilestone { bounty: u32 },
    /// The career bounty rose by `amount` since the last frame.
    BountyGained { amount: u32 },
    /// A tyre was punctured (or blew).
    TirePunctured,
    /// The body damage passed the heavy-damage mark.
    HeavyDamage,
    /// The car no longer drives: the engine blew or the car is wrecked.
    VehicleCritical,
    /// The nitrous began to burn.
    NitrousOn,
}
