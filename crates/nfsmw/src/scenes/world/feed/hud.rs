//! The HUD's events (`hud::HudEvent`), once per change. Three sources feed a frame's list:
//!
//! - the chase's notices (`pursuit.rs`), the same the mods hear: the heat rising, the bust and the escape;
//! - the places where the game raises an event itself: the race's checkpoints, laps and wrong-way zone, and the speed
//!   traps (`race/flow.rs`, `career/speedtraps.rs`);
//! - the diff of a small snapshot taken once a frame (`world.rs`): the pursuit phase, the roadblock report, the
//!   helicopter's sight, the race place, the near misses, the bounty, the car's condition and the nitrous.
//!
//! The first snapshot only sets the baseline: a world that opens into a state does not announce it. An identical state
//! over any number of frames makes no event. Near misses are throttled to one every [`NEAR_MISS_GAP`] seconds.

use blackbox_race::RaceEvent;

use super::pursuit::Notice;
use crate::hud::{HudEvent, PursuitPhase};

/// Seconds between two near-miss events.
pub const NEAR_MISS_GAP: f32 = 1.5;
/// The bounty milestones are the multiples of this. A presentation step, not a game value.
pub const BOUNTY_STEP: u32 = 10_000;

/// What the HUD's events are diffed from, once a frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(in crate::scenes::world) struct FeedSnapshot {
    pub phase: PursuitPhase,
    /// Dispatch has a roadblock standing in the chase.
    pub roadblock_reported: bool,
    /// The helicopter has the player in sight.
    pub helicopter_tracking: bool,
    /// The player's place in the race; `None` outside one.
    pub place: Option<u32>,
    /// The near misses so far: a count that only grows while the traffic lives.
    pub near_misses: u32,
    /// The career bounty; `None` outside career mode.
    pub bounty: Option<u32>,
    /// The tyres punctured or blown.
    pub damaged_tires: usize,
    /// The body damage is past the heavy-damage mark.
    pub heavy_damage: bool,
    /// The car no longer drives.
    pub critical: bool,
    pub nos_burning: bool,
}

/// The HUD's events of the frame, and what the next frame's are diffed against.
#[derive(Debug, Default)]
pub(in crate::scenes::world) struct HudFeed {
    /// Counts the frames, so a reader that sees the same list twice knows it (`HudState::events_frame`).
    frame: u64,
    events: Vec<HudEvent>,
    before: Option<FeedSnapshot>,
    /// Seconds before another near miss may be told.
    near_miss_wait: f32,
}

impl HudFeed {
    /// Start a frame: the last frame's events are gone.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.events.clear();
    }

    /// An event the game raised itself this frame.
    pub fn push(&mut self, event: HudEvent) {
        self.events.push(event);
    }

    /// The chase's notices of this frame, as the mods heard them.
    pub fn notices(&mut self, notices: &[Notice]) {
        self.events.extend(notices.iter().filter_map(from_notice));
    }

    /// One of the race engine's events for the player, with the checkpoints of a lap and the laps of the race.
    pub fn race_event(&mut self, event: &RaceEvent, checkpoints: usize, laps: u32) {
        self.events.extend(from_race_event(event, checkpoints, laps));
    }

    /// The frame's snapshot, `dt` seconds after the last: the changes since the last snapshot become events.
    pub fn observe(&mut self, now: FeedSnapshot, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.near_miss_wait = (self.near_miss_wait - dt).max(0.0);
        }
        let Some(before) = self.before.replace(now) else { return };
        self.events.extend(edges(&before, &now));
        if now.near_misses > before.near_misses && self.near_miss_wait <= 0.0 {
            self.events.push(HudEvent::NearMiss);
            self.near_miss_wait = NEAR_MISS_GAP;
        }
    }

    /// The frame's number: it changes once a frame.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// The events of this frame, in the order they were raised.
    pub fn events(&self) -> &[HudEvent] {
        &self.events
    }
}

/// The HUD's event for a chase notice: the heat rising, the bust and the escape. The start and end of a chase and the
/// infractions are the phase's and the mods' business.
fn from_notice(notice: &Notice) -> Option<HudEvent> {
    match *notice {
        Notice::HeatChanged { level, old_heat, new_heat } if new_heat > old_heat => Some(HudEvent::HeatUp { level }),
        Notice::Busted { .. } => Some(HudEvent::Busted),
        Notice::Escaped { .. } => Some(HudEvent::Escaped),
        _ => None,
    }
}

/// The HUD's event for one of the race engine's events: the player's checkpoints, laps, speed traps and the wrong-way
/// zone. Another racer's events, and the rest, are none.
fn from_race_event(event: &RaceEvent, checkpoints: usize, laps: u32) -> Option<HudEvent> {
    match *event {
        RaceEvent::Checkpoint { racer: 0, number } => {
            Some(HudEvent::Checkpoint { number: number as u32, total: checkpoints as u32 })
        }
        // `lap` laps are done: the next one begins.
        RaceEvent::Lap { racer: 0, lap } => Some(HudEvent::Lap { lap: lap + 1, laps }),
        RaceEvent::SpeedTrap { racer: 0, kmh, .. } => Some(HudEvent::SpeedTrap { kmh, passed: true }),
        RaceEvent::Reset { racer: 0 } => Some(HudEvent::WrongWay),
        _ => None,
    }
}

/// The events of the change from `before` to `now` (without the near misses, which [`HudFeed::observe`] throttles).
fn edges(before: &FeedSnapshot, now: &FeedSnapshot) -> Vec<HudEvent> {
    let mut out = Vec::new();
    if before.phase != now.phase {
        out.push(HudEvent::PhaseChanged { from: before.phase, to: now.phase });
    }
    if before.phase == PursuitPhase::Active && now.phase == PursuitPhase::Search {
        out.push(HudEvent::ContactLost);
    }
    if before.phase != PursuitPhase::Cooldown && now.phase == PursuitPhase::Cooldown {
        out.push(HudEvent::CooldownStarted);
    }
    if !before.roadblock_reported && now.roadblock_reported {
        out.push(HudEvent::RoadblockReported);
    }
    if !before.helicopter_tracking && now.helicopter_tracking {
        out.push(HudEvent::HelicopterTracking);
    }
    if let (Some(from), Some(to)) = (before.place, now.place)
        && from != to
    {
        out.push(HudEvent::PositionChanged { from, to });
    }
    if let Some(bounty) = milestone(before.bounty, now.bounty) {
        out.push(HudEvent::BountyMilestone { bounty });
    }
    if now.damaged_tires > before.damaged_tires {
        out.push(HudEvent::TirePunctured);
    }
    if !before.heavy_damage && now.heavy_damage {
        out.push(HudEvent::HeavyDamage);
    }
    if !before.critical && now.critical {
        out.push(HudEvent::VehicleCritical);
    }
    if !before.nos_burning && now.nos_burning {
        out.push(HudEvent::NitrousOn);
    }
    out
}

/// The highest bounty milestone (a multiple of [`BOUNTY_STEP`]) passed on the way from `before` up to `now`; `None`
/// when none was passed, the bounty fell, or either is unknown.
fn milestone(before: Option<u32>, now: Option<u32>) -> Option<u32> {
    let (before, now) = (before?, now?);
    let reached = now / BOUNTY_STEP;
    (reached > before / BOUNTY_STEP).then_some(reached * BOUNTY_STEP)
}

#[cfg(test)]
#[path = "hud_tests.rs"]
mod tests;
