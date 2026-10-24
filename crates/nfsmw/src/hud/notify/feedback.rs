//! The feedback lane: short chips just above the cluster for the driving's passing news, apart from the notices so that
//! they never compete with a warning (`docs/specs/custom-hud/feedback.md` section 1). The lane holds one chip at a time:
//!
//! - **near miss:** each one while the chip is up refreshes it and counts up (`Near miss ×3`) without replaying its
//!   entry; the world already throttles the events to one every 1.5 s (`scenes/world/feed/hud.rs`);
//! - **speed trap:** the speed through the trap, or too slow;
//! - **bounty gain:** a delta that counts up over [`COUNT_UP`] seconds, held for [`BOUNTY_HOLD`], then gone; gains while
//!   it is up add up, and the count runs on from the value shown.
//!
//! A chip of another kind takes the lane once the one up has been there [`MIN_UP`] seconds; until then it waits (the
//! latest of its kind wins). The nitrous coming on is no chip: the cluster's nitrous bar pulses once, from
//! [`FeedbackLane::nitrous_age`].
//!
//! The lane runs on its own clock, advanced by [`FeedbackLane::tick`], like the notification queue.

use crate::hud::HudEvent;

/// Seconds a near-miss chip stays after its last near miss: a little longer than the world's 1.5 s throttle, so that a
/// run of near misses counts up on one chip.
pub const NEAR_MISS_HOLD: f32 = 2.0;
/// Seconds a speed-trap chip stays.
pub const SPEED_TRAP_HOLD: f32 = 2.5;
/// Seconds the bounty delta takes to count up to its value.
pub const COUNT_UP: f32 = 0.6;
/// Seconds the bounty delta stays once counted up (the motion table's `delta_hold`).
pub const BOUNTY_HOLD: f32 = 2.0;
/// Seconds a chip is up before a chip of another kind may take its place.
pub const MIN_UP: f32 = 0.5;
/// Seconds after the nitrous came on that [`FeedbackLane::nitrous_age`] still reports it.
pub const NITROUS_PULSE: f32 = 1.0;

/// What a chip shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipKind {
    NearMiss,
    SpeedTrap,
    Bounty,
}

impl ChipKind {
    /// The chip kind of `event`; `None` for an event the lane does not show.
    pub fn of(event: &HudEvent) -> Option<Self> {
        match event {
            HudEvent::NearMiss => Some(Self::NearMiss),
            HudEvent::SpeedTrap { .. } => Some(Self::SpeedTrap),
            HudEvent::BountyGained { .. } => Some(Self::Bounty),
            _ => None,
        }
    }

    /// Seconds the chip stays after its last push.
    fn hold(self) -> f32 {
        match self {
            Self::NearMiss => NEAR_MISS_HOLD,
            Self::SpeedTrap => SPEED_TRAP_HOLD,
            Self::Bounty => COUNT_UP + BOUNTY_HOLD,
        }
    }
}

/// The chip in the lane, or the one waiting for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chip {
    pub kind: ChipKind,
    /// The latest event folded in: the one whose values it shows.
    pub event: HudEvent,
    /// The lane's clock when it came up: its entry runs from here, and a repeat does not move it.
    pub started: f32,
    /// The lane's clock of the latest push folded in.
    pub refreshed: f32,
    /// The lane's clock at which it goes.
    pub expires: f32,
    /// The pushes folded into it, 1 for the first.
    pub count: u32,
    /// The bounty gained over the chip's life (0 for the other kinds).
    pub total: u32,
    /// The bounty the chip showed when the latest gain came: the count-up runs from here to `total`.
    pub from: u32,
}

impl Chip {
    fn new(kind: ChipKind, event: HudEvent, now: f32) -> Self {
        let total = gain(&event);
        Self { kind, event, started: now, refreshed: now, expires: now + kind.hold(), count: 1, total, from: 0 }
    }

    /// Fold a repeat of the same kind in: the values are the new event's, the time is pushed out, the entry stays.
    fn fold(&mut self, event: HudEvent, now: f32) {
        self.from = self.bounty_at(now, true);
        self.total = self.total.saturating_add(gain(&event));
        self.event = event;
        self.refreshed = now;
        self.expires = now + self.kind.hold();
        self.count += 1;
    }

    /// Bring a waiting chip up at `now`: its time starts again from there.
    fn bring_up(&mut self, now: f32) {
        self.started = now;
        self.refreshed = now;
        self.expires = now + self.kind.hold();
    }

    /// The bounty the chip shows at `now`: counting up from `from` to `total` over [`COUNT_UP`] seconds after the
    /// latest gain, or `total` at once when `count_up` is off (motion reduction).
    pub fn bounty_at(&self, now: f32, count_up: bool) -> u32 {
        if !count_up || self.total <= self.from {
            return self.total;
        }
        let progress = ((now - self.refreshed) / COUNT_UP).clamp(0.0, 1.0);
        self.from + ((self.total - self.from) as f32 * progress).round() as u32
    }
}

/// The bounty an event gained: its amount for a bounty gain, 0 for any other.
fn gain(event: &HudEvent) -> u32 {
    match event {
        HudEvent::BountyGained { amount } => *amount,
        _ => 0,
    }
}

/// The lane: the chip up, the one waiting, and when the nitrous last came on.
#[derive(Debug, Clone, Default)]
pub struct FeedbackLane {
    clock: f32,
    chip: Option<Chip>,
    next: Option<Chip>,
    nitrous: Option<f32>,
}

impl FeedbackLane {
    /// Take one of the frame's events. An event the lane does not show is ignored.
    pub fn push(&mut self, event: HudEvent) {
        let now = self.clock;
        if event == HudEvent::NitrousOn {
            self.nitrous = Some(now);
            return;
        }
        let Some(kind) = ChipKind::of(&event) else { return };
        if let Some(chip) = self.chip.as_mut().filter(|chip| chip.kind == kind && chip.expires > now) {
            chip.fold(event, now);
            return;
        }
        if let Some(next) = self.next.as_mut().filter(|next| next.kind == kind) {
            next.fold(event, now);
            return;
        }
        let chip = Chip::new(kind, event, now);
        if self.chip.is_some_and(|up| now - up.started < MIN_UP) {
            self.next = Some(chip);
            return;
        }
        self.chip = Some(chip);
        self.next = None;
    }

    /// Advance the lane by `dt` seconds: an expired chip goes, and a waiting one comes up once the lane is free or the
    /// chip up has had its [`MIN_UP`].
    pub fn tick(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0.0 {
            self.clock += dt;
        }
        let now = self.clock;
        self.chip = self.chip.filter(|chip| chip.expires > now);
        self.nitrous = self.nitrous.filter(|since| now - since < NITROUS_PULSE);
        let free = self.chip.is_none_or(|chip| now - chip.started >= MIN_UP);
        if !free {
            return;
        }
        let Some(mut next) = self.next.take() else { return };
        next.bring_up(now);
        self.chip = Some(next);
    }

    /// The chip up, if any.
    pub fn chip(&self) -> Option<&Chip> {
        self.chip.as_ref()
    }

    /// The lane's clock: `Chip::started`, `refreshed` and `expires` are on it.
    pub fn clock(&self) -> f32 {
        self.clock
    }

    /// Seconds since the nitrous came on, for the cluster's one pulse; `None` once [`NITROUS_PULSE`] has passed.
    pub fn nitrous_age(&self) -> Option<f32> {
        self.nitrous.map(|since| self.clock - since)
    }
}
