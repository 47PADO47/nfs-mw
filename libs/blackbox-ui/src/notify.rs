//! A small notification queue: what a HUD shows of the game's events, a few at a time, the most urgent first.
//! Engine-generic: the host names its notices with its own key type and gives each a priority, a duration and a dedupe
//! window; the queue decides what is visible.
//!
//! The rules:
//!
//! - **Capacity.** At most [`CAPACITY`] notices are visible (or the capacity given to [`NotifyQueue::with_capacity`]);
//!   the rest wait, the most urgent first, then the oldest.
//! - **Preemption.** A [`Priority::Critical`] or [`Priority::High`] notice that finds no room evicts the least urgent
//!   visible notice that is less urgent than itself (the oldest of those on a tie). The evicted notice is dropped: it
//!   would be stale when it came back.
//! - **Deferral.** While the game is intense (the host says so each [`NotifyQueue::tick`]), [`Priority::Low`] and
//!   [`Priority::Background`] notices wait, even with room.
//! - **Dedupe.** A push of a key pushed less than the notice's [`Dedupe::window`] ago is folded into the first: a
//!   visible one keeps its `started` time (its animation does not restart) and has its expiry pushed out and its
//!   payload replaced; a waiting one has its payload replaced; one that has already gone is not shown again. A push
//!   after the window is a new notice, except that a key is never visible twice: a visible one is refreshed.
//! - **Expiry.** A visible notice goes at the end of its duration (`f32::INFINITY` keeps it until it is dismissed); a
//!   waiting one with a `max_wait` goes when it has waited that long.
//! - **Dismissal.** [`NotifyQueue::dismiss`] takes a key off the screen and out of the queue; the dedupe window still
//!   holds it back.

/// The notices visible at once, unless the queue is made with another capacity.
pub const CAPACITY: usize = 3;

/// How urgent a notice is, the most urgent first (`Critical < High < ...` in the ordering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
    Background,
}

impl Priority {
    /// Critical and High interrupt: with no room they evict a less urgent visible notice.
    pub fn preempts(self) -> bool {
        matches!(self, Priority::Critical | Priority::High)
    }

    /// Low and Background wait while the game is intense.
    pub fn deferrable(self) -> bool {
        matches!(self, Priority::Low | Priority::Background)
    }
}

/// How repeats of a key are folded (see the module).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dedupe {
    /// Seconds after a key's last push within which another push of it is a repeat.
    pub window: f32,
}

/// A notice to show: its key, what it carries, how urgent it is and how long it stays up.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice<K, P = ()> {
    pub key: K,
    /// What the host draws it with (the values of its text); a repeat replaces it.
    pub payload: P,
    pub priority: Priority,
    /// Seconds it stays visible; `f32::INFINITY` until it is dismissed.
    pub duration: f32,
    pub dedupe: Dedupe,
    /// Seconds it may wait for room before it goes stale and is dropped; `None` waits as long as it takes.
    pub max_wait: Option<f32>,
}

/// A visible notice.
#[derive(Debug, Clone, PartialEq)]
pub struct Active<K, P = ()> {
    pub key: K,
    pub payload: P,
    pub priority: Priority,
    /// The queue's clock when it came up: its animation runs from here, and a repeat does not move it.
    pub started: f32,
    /// The queue's clock at which it goes.
    pub expires: f32,
    /// The pushes folded into it, 1 for the first.
    pub count: u32,
}

/// A notice that waits for room (or for the game to calm down).
#[derive(Debug, Clone, PartialEq)]
struct Waiting<K, P> {
    notice: Notice<K, P>,
    /// The queue's clock when it was first pushed.
    since: f32,
    count: u32,
}

/// A key's last push, kept for its dedupe window.
#[derive(Debug, Clone, PartialEq)]
struct Recent<K> {
    key: K,
    at: f32,
    window: f32,
}

/// The queue. See the module for its rules.
#[derive(Debug, Clone)]
pub struct NotifyQueue<K, P = ()> {
    capacity: usize,
    clock: f32,
    intense: bool,
    visible: Vec<Active<K, P>>,
    waiting: Vec<Waiting<K, P>>,
    recent: Vec<Recent<K>>,
}

impl<K: Eq + Clone, P: Clone> Default for NotifyQueue<K, P> {
    fn default() -> Self {
        Self::with_capacity(CAPACITY)
    }
}

impl<K: Eq + Clone, P: Clone> NotifyQueue<K, P> {
    /// An empty queue showing at most [`CAPACITY`] notices.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty queue showing at most `capacity` notices (at least one).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            clock: 0.0,
            intense: false,
            visible: Vec::new(),
            waiting: Vec::new(),
            recent: Vec::new(),
        }
    }

    /// The notices on the screen, in the order they came up.
    pub fn visible(&self) -> &[Active<K, P>] {
        &self.visible
    }

    /// How many notices wait.
    pub fn waiting(&self) -> usize {
        self.waiting.len()
    }

    /// The queue's clock: the seconds ticked so far. `Active::started` and `Active::expires` are on it.
    pub fn clock(&self) -> f32 {
        self.clock
    }

    /// Push a notice. It comes up now when there is room (or it preempts), and waits otherwise; a repeat is folded.
    pub fn push(&mut self, notice: Notice<K, P>) {
        let repeat = self.recent.iter().any(|r| r.key == notice.key && self.clock - r.at < r.window);
        self.note_push(&notice);
        if let Some(active) = self.visible.iter_mut().find(|a| a.key == notice.key) {
            active.expires = active.expires.max(self.clock + notice.duration);
            active.payload = notice.payload;
            active.count += 1;
            return;
        }
        if let Some(waiting) = self.waiting.iter_mut().find(|w| w.notice.key == notice.key) {
            waiting.count += 1;
            waiting.notice.payload = notice.payload;
            return;
        }
        if repeat {
            return;
        }
        if notice.priority.deferrable() && self.intense {
            self.wait(notice);
            return;
        }
        if self.visible.len() < self.capacity {
            self.show(notice, 1);
            return;
        }
        if notice.priority.preempts()
            && let Some(at) = self.evictable(notice.priority)
        {
            self.visible.remove(at);
            self.show(notice, 1);
            return;
        }
        self.wait(notice);
    }

    /// Advance the clock by `dt` seconds. `intense` holds Low and Background notices back until it is false again.
    /// Expired notices go, stale waiting ones are dropped, and waiting ones come up into the room left.
    pub fn tick(&mut self, dt: f32, intense: bool) {
        if dt.is_finite() && dt > 0.0 {
            self.clock += dt;
        }
        self.intense = intense;
        let clock = self.clock;
        self.visible.retain(|active| active.expires > clock);
        self.waiting.retain(|w| w.notice.max_wait.is_none_or(|limit| clock - w.since < limit));
        self.recent.retain(|r| clock - r.at < r.window);
        while self.visible.len() < self.capacity {
            let Some(at) = self.next_waiting() else { break };
            let waiting = self.waiting.remove(at);
            self.show(waiting.notice, waiting.count);
        }
    }

    /// Take `key` off the screen and out of the queue. Returns whether it was there.
    pub fn dismiss(&mut self, key: &K) -> bool {
        let before = self.visible.len() + self.waiting.len();
        self.visible.retain(|active| active.key != *key);
        self.waiting.retain(|w| w.notice.key != *key);
        before != self.visible.len() + self.waiting.len()
    }

    /// Drop every notice, visible and waiting, and forget the recent keys.
    pub fn clear(&mut self) {
        self.visible.clear();
        self.waiting.clear();
        self.recent.clear();
    }

    /// Remember the push of a key for its dedupe window.
    fn note_push(&mut self, notice: &Notice<K, P>) {
        let window = notice.dedupe.window.max(0.0);
        match self.recent.iter_mut().find(|r| r.key == notice.key) {
            Some(recent) => {
                recent.at = self.clock;
                recent.window = window;
            }
            None => self.recent.push(Recent { key: notice.key.clone(), at: self.clock, window }),
        }
    }

    fn show(&mut self, notice: Notice<K, P>, count: u32) {
        self.visible.push(Active {
            expires: self.clock + notice.duration,
            key: notice.key,
            payload: notice.payload,
            priority: notice.priority,
            started: self.clock,
            count,
        });
    }

    fn wait(&mut self, notice: Notice<K, P>) {
        self.waiting.push(Waiting { notice, since: self.clock, count: 1 });
    }

    /// The visible notice a notice of `priority` may evict: the least urgent of those less urgent than it, the oldest
    /// on a tie.
    fn evictable(&self, priority: Priority) -> Option<usize> {
        let candidates = self.visible.iter().enumerate().filter(|(_, active)| active.priority > priority);
        let worst =
            candidates.max_by(|(_, a), (_, b)| a.priority.cmp(&b.priority).then(b.started.total_cmp(&a.started)));
        worst.map(|(at, _)| at)
    }

    /// The waiting notice to come up next: the most urgent that may show now, the oldest on a tie.
    fn next_waiting(&self) -> Option<usize> {
        let allowed =
            self.waiting.iter().enumerate().filter(|(_, w)| !(self.intense && w.notice.priority.deferrable()));
        let best = allowed
            .min_by(|(_, a), (_, b)| a.notice.priority.cmp(&b.notice.priority).then(a.since.total_cmp(&b.since)));
        best.map(|(at, _)| at)
    }
}

#[cfg(test)]
mod tests;
