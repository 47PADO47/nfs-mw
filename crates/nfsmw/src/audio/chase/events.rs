//! What happened in the world that the cops' radio remarks on: crashes, jumps, hits on cops, a roadblock's outcome, a
//! backup call. The world pushes these as they happen (`scenes/world/ai/police/speech_events.rs`) and the feed carries
//! the last few, each with a number; the radio takes the ones it has not seen (`radio/world_notices.rs`) and turns them
//! into the machine's notices. The numbers are physical (metres per second); the thresholds of the speech tuning are
//! applied by the radio, so the world does not need the database.

use blackbox_pursuit::speech::{Backup, Impact, RoadblockHit};

/// The events the feed keeps. A frame never raises more than a handful; the oldest drop first.
pub const MAX_EVENTS: usize = 24;

/// How hard a collision was and what it did to the player's car.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Crash {
    /// The speed the two bodies lost between them, metres per second (the original's `impulseA + impulseB`).
    pub delta_v: f32,
    /// The speed the bodies closed at, metres per second.
    pub closing: f32,
    /// The player's speed before and after the step it hit in, metres per second.
    pub speed_before: f32,
    pub speed_after: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WorldEvent {
    /// The player's car hit the static world (a wall, a barrier, a rail).
    PlayerHitWorld(Crash),
    /// The player's car hit a prop (a sign, a fence post): it counts as a crash but is nobody's remark.
    PlayerHitProp(Crash),
    /// The player's car hit a traffic car; `semi` for a tractor or its trailer.
    PlayerHitTraffic {
        crash: Crash,
        semi: bool,
    },
    /// The player's car hit a cop's. `impact` says how they met (`Impact::None` when it was not the player's ram) and
    /// `deliberate` is false for a cop that was braking, in a roadblock, or an SUV.
    PlayerHitCop {
        cop: u32,
        crash: Crash,
        impact: Impact,
        deliberate: bool,
    },
    CopHitTraffic {
        cop: u32,
        closing: f32,
    },
    CopHitWorld {
        cop: u32,
        delta_v: f32,
        closing: f32,
    },
    /// The player landed after this long in the air, at most this high above the ground.
    Airborne {
        altitude: f32,
        secs: f32,
    },
    /// The player passed this cop close and fast.
    BlewByCop {
        cop: u32,
    },
    /// The player drove into (true) or out of (false) a tunnel or other enclosure.
    Tunnel {
        inside: bool,
    },
    /// A gas station blew up at this physics-space position.
    GasStation {
        by_player: bool,
        at: [f32; 3],
    },
    /// A roadblock was built; the offset of its spike strip from the road's centre when it has one.
    RoadblockBuilt {
        spike_offset: Option<i32>,
    },
    RoadblockDodged,
    /// The player hit something of the roadblock: a spike strip, another object, a car.
    RoadblockHit(RoadblockHit),
    /// A spike strip burst a tire.
    TireBlown,
    BackupRequested(Backup),
    BackupDenied(Backup),
}

/// The most recent events, oldest first, each with a number that only grows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventLog {
    items: [(u32, WorldEvent); MAX_EVENTS],
    len: usize,
    next: u32,
}

impl Default for EventLog {
    fn default() -> Self {
        Self { items: [(0, WorldEvent::RoadblockDodged); MAX_EVENTS], len: 0, next: 1 }
    }
}

impl EventLog {
    pub fn push(&mut self, event: WorldEvent) {
        if self.len == MAX_EVENTS {
            self.items.copy_within(1.., 0);
            self.len -= 1;
        }
        self.items[self.len] = (self.next, event);
        self.len += 1;
        self.next += 1;
    }

    /// The number of the last event pushed (0 before any).
    pub fn latest(&self) -> u32 {
        self.next - 1
    }

    /// The events with a number above `seen`, oldest first.
    pub fn since(&self, seen: u32) -> impl Iterator<Item = &WorldEvent> {
        self.items[..self.len].iter().filter(move |(seq, _)| *seq > seen).map(|(_, event)| event)
    }

    /// Every event the log holds, oldest first.
    pub fn all(&self) -> impl Iterator<Item = &WorldEvent> {
        self.since(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_come_back_in_order_once() {
        let mut log = EventLog::default();
        log.push(WorldEvent::RoadblockDodged);
        log.push(WorldEvent::TireBlown);
        let seen = log.latest();
        log.push(WorldEvent::Tunnel { inside: true });
        assert_eq!(log.since(seen).count(), 1);
        assert_eq!(log.since(0).count(), 3);
        assert_eq!(log.since(log.latest()).count(), 0);
    }

    #[test]
    fn the_oldest_drop_when_the_log_is_full() {
        let mut log = EventLog::default();
        for i in 0..MAX_EVENTS + 5 {
            log.push(WorldEvent::BlewByCop { cop: i as u32 });
        }
        assert_eq!(log.all().count(), MAX_EVENTS);
        assert_eq!(log.all().next(), Some(&WorldEvent::BlewByCop { cop: 5 }));
        assert_eq!(log.latest(), (MAX_EVENTS + 5) as u32);
    }
}
