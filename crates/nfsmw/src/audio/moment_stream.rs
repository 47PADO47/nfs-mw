//! The one moment stream every moment recording shares (breaker, booth and car crashes, petrol blast, pursuit
//! breaker sting, thunder): admission by priority, interruption, the breaker-marker queue and its release.
//! Spec: `scenery-effect-audio.md` ("Shared moment stream").
use blackbox_attrib::vlt_hash;
use glam::Vec3;

/// A player within this many metres of a breaker marker on both ground axes queues its recording.
pub(super) const QUEUE_RADIUS: f32 = 20.0;
/// `unpause` releases the queued recording of a marker within this many metres on both ground axes.
const UNPAUSE_RADIUS: f32 = 25.0;
/// The same moment is held back this long after it starts playing.
const RETRIGGER_SECONDS: f32 = 2.0;
/// A positioned moment stops when every player is farther away than this.
const FOLLOW_RANGE: f32 = 120.0;
/// Moments that restart on every request: each booth of a chain, each window.
const ALWAYS_RESTART: [u32; 2] = [vlt_hash("tollbooth"), vlt_hash("window")];
pub(super) const UNPAUSE: u32 = vlt_hash("unpause");

/// The voice that plays the current moment; committing another moment stops it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Owner {
    Petrol,
    Breaker,
    Thunder,
    World,
}

/// A moment's `strmpriority` and `CanInterupt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(super) struct Rank {
    pub priority: i8,
    pub can_interrupt: bool,
}

#[derive(Clone, Copy, Debug)]
struct Current {
    key: u32,
    rank: Rank,
    /// `None` while queued: nothing plays yet.
    owner: Option<Owner>,
    /// `None` for a moment heard at the listener.
    point: Option<Vec3>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision {
    Refuse,
    /// The request names the queued moment: play it now, where it was queued.
    Release,
    /// Play it, stopping the voice of the moment it replaces.
    Start {
        interrupt: Option<Owner>,
    },
}

/// What a frame of the stream asks the host to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Tick {
    pub stop: Option<Owner>,
    pub queued: Option<u32>,
}

#[derive(Default)]
pub(super) struct Arbiter {
    current: Option<Current>,
    /// The marker whose recording is queued, an index of `markers`.
    held: Option<usize>,
    retrigger: f32,
    /// Breaker markers (render space) and the moment each queues.
    markers: Vec<(Vec3, u32)>,
}

/// Squared ground distance (render x and y).
fn ground(a: Vec3, b: Vec3) -> (f32, f32) {
    ((a.x - b.x).abs(), (a.y - b.y).abs())
}

impl Arbiter {
    pub fn set_markers(&mut self, markers: Vec<(Vec3, u32)>) {
        self.clear();
        self.markers = markers;
    }

    pub fn markers(&self) -> &[(Vec3, u32)] {
        &self.markers
    }

    pub fn current(&self) -> Option<u32> {
        self.current.map(|c| c.key)
    }

    pub fn owner(&self) -> Option<Owner> {
        self.current.and_then(|c| c.owner)
    }

    pub fn is_held(&self) -> bool {
        self.held.is_some()
    }

    /// Where the current moment plays (a queued one: its marker).
    pub fn point(&self) -> Option<Vec3> {
        self.current.and_then(|c| c.point)
    }

    /// Whether a request may take the stream. `queueing` requests come from the marker queue, at `distance_sq` from
    /// the nearest player; `blocked` is another stream (a cinematic's) playing.
    pub fn admits(
        &self,
        key: u32,
        rank: Rank,
        queueing: bool,
        distance_sq: f32,
        players: &[Vec3],
        blocked: bool,
    ) -> bool {
        let Some(current) = self.current else { return !blocked };
        if current.key == key && self.retrigger > 0.0 && !ALWAYS_RESTART.contains(&key) {
            return false;
        }
        if let (true, Some(held)) = (queueing, self.held) {
            let at = self.markers[held].0;
            let closer = players.iter().any(|p| {
                let (x, y) = ground(*p, at);
                x * x + y * y < distance_sq
            });
            if closer {
                return false;
            }
        }
        if blocked || (queueing && !rank.can_interrupt) {
            return false;
        }
        current.rank.priority <= rank.priority
    }

    /// A moment request (an event, a crash, the breaker sting, thunder).
    pub fn receive(&self, key: u32, rank: Rank, players: &[Vec3], blocked: bool) -> Decision {
        if self.held.is_some() && self.current.is_some_and(|c| c.key == key) {
            return Decision::Release;
        }
        if !self.admits(key, rank, false, 0.0, players, blocked) {
            return Decision::Refuse;
        }
        Decision::Start { interrupt: self.owner() }
    }

    /// The breaker marker `unpause` at `position` releases (the last in table order within reach), and its moment.
    pub fn unpause_target(&self, position: Vec3) -> Option<(u32, Vec3)> {
        self.markers.iter().rev().find_map(|&(at, key)| {
            let (x, y) = ground(position, at);
            (x < UNPAUSE_RADIUS && y < UNPAUSE_RADIUS).then_some((key, at))
        })
    }

    /// `owner` started playing `key`: it holds the stream, and the same moment waits before it may restart.
    pub fn started(&mut self, owner: Owner, key: u32, rank: Rank, point: Option<Vec3>) {
        self.current = Some(Current { key, rank, owner: Some(owner), point });
        self.held = None;
        self.retrigger = RETRIGGER_SECONDS;
    }

    /// `owner` stopped its voice on its own (the breaker ended, the scene changed).
    pub fn stopped(&mut self, owner: Owner) {
        if self.owner() == Some(owner) {
            self.current = None;
        }
    }

    pub fn clear(&mut self) {
        self.current = None;
        self.held = None;
        self.retrigger = 0.0;
    }

    /// One frame: the retrigger clock, a queued moment the players left, a finished or distant one, and the marker
    /// queue. `rank_of` gives a queued moment's rank (`None` when the install lacks it).
    pub fn update(
        &mut self,
        dt: f32,
        players: &[Vec3],
        playing: bool,
        blocked: bool,
        rank_of: impl Fn(u32) -> Option<Rank>,
    ) -> Tick {
        let mut tick = Tick::default();
        self.retrigger = (self.retrigger - dt).max(0.0);
        if let Some(current) = self.current {
            if let Some(held) = self.held {
                let at = self.markers[held].0;
                let near = players.iter().any(|p| {
                    let (x, y) = ground(*p, at);
                    x < QUEUE_RADIUS || y < QUEUE_RADIUS
                });
                if !near {
                    self.clear_queued();
                }
            }
            if self.held.is_none() && !playing {
                self.current = None;
            }
            let far = current.point.is_some_and(|at| players.iter().all(|p| p.distance(at) > FOLLOW_RANGE));
            if self.held.is_none() && self.current.is_some() && far {
                tick.stop = current.owner;
                self.current = None;
            }
        }
        for index in 0..self.markers.len() {
            let (at, key) = self.markers[index];
            if self.current() == Some(key) {
                continue;
            }
            for &player in players {
                let (x, y) = ground(player, at);
                if x > QUEUE_RADIUS || y > QUEUE_RADIUS {
                    continue;
                }
                let Some(rank) = rank_of(key) else { continue };
                if !self.admits(key, rank, true, x * x + y * y, players, blocked) {
                    continue;
                }
                tick.stop = tick.stop.or(self.owner());
                self.current = Some(Current { key, rank, owner: None, point: Some(at) });
                self.held = Some(index);
                tick.queued = Some(key);
            }
        }
        tick
    }

    fn clear_queued(&mut self) {
        self.current = None;
        self.held = None;
    }
}

mod host;
pub(super) use host::Stream;

#[cfg(test)]
mod installed;
#[cfg(test)]
mod tests;
