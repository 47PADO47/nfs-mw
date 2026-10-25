//! The catalogue of the HUD's notifications: for each kind of [`HudEvent`], what the player is told and how. Each entry
//! names its id (the queue's key), its category, its priority, its text and optional second line (Fluent ids in
//! `i18n/en/hud.ftl`), its icon, how long it stays, what dismisses it, the sound and the pad rumble it asks for (played
//! by the HUD's audio and haptics, which read these ids), and its dedupe window.
//!
//! The priorities follow the brief's examples: a hazard on the road ahead is Critical (the roadblock), a mechanical
//! warning High (a tyre, heavy damage, a disabled car), a missed checkpoint Medium (the wrong-way zone, with the race's
//! other updates), and the passing news Low (near misses, bounty milestones). The race's and the chase's notices are
//! Medium or above so that they show while the game is intense, when Low waits.
//!
//! A kind with no text is feedback only: it has its sound and rumble but no notice (the nitrous, whose bar pulses). A
//! kind in the feedback lane (the near misses, the speed traps, the bounty gains) is a short chip above the cluster
//! (`feedback.rs`), not a notice of the queue, so it never competes with a warning
//! (`docs/specs/custom-hud/notifications.md` section 5). The phase changes have no entry: they change the HUD's layout
//! instead.

use blackbox_ui::notify::{Dedupe, Notice, Priority};

use crate::hud::{HudEvent, PursuitPhase};

/// A notification of the catalogue: the queue's key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoticeId {
    HeatUp,
    Roadblock,
    Helicopter,
    ContactLost,
    Cooldown,
    Escaped,
    Busted,
    Checkpoint,
    WrongWay,
    Lap,
    Position,
    NearMiss,
    SpeedTrap,
    SpeedTrapSlow,
    Bounty,
    BountyGain,
    TirePunctured,
    HeavyDamage,
    VehicleCritical,
    Nitrous,
}

/// What a notification is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Pursuit,
    Race,
    Vehicle,
    Driving,
    Career,
}

/// What takes a notification away, besides the dismiss action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dismissal {
    /// The end of its duration.
    Timed,
    /// The chase leaving this phase, or the end of its duration if that comes first.
    WhilePhase(PursuitPhase),
}

/// The sound a notification asks for. The HUD's audio picks the bank trigger for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCue {
    /// The chase grows: the heat, the helicopter.
    Escalation,
    /// A hazard ahead: the roadblock.
    Hazard,
    ContactLost,
    Cooldown,
    Escaped,
    Busted,
    Checkpoint,
    MissedCheckpoint,
    CriticalDamage,
    Nitrous,
}

/// The pad rumble a notification asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Haptic {
    Light,
    Strong,
}

/// One entry of the catalogue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoticeSpec {
    pub id: NoticeId,
    pub category: Category,
    pub priority: Priority,
    /// The Fluent id of its text; `None` for feedback only (a sound or a rumble, no notice).
    pub text: Option<&'static str>,
    /// The Fluent id of its second line.
    pub secondary: Option<&'static str>,
    /// The icon the drawing shows beside the text.
    pub icon: &'static str,
    /// Seconds it stays up.
    pub duration: f32,
    pub dismissal: Dismissal,
    pub audio: Option<AudioCue>,
    pub haptic: Option<Haptic>,
    /// Seconds after a push within which the same id is a repeat (see `blackbox_ui::notify`).
    pub dedupe: f32,
    /// Seconds it may wait for room before it is stale; `None` waits.
    pub max_wait: Option<f32>,
    /// Shown as a chip of the feedback lane rather than a notice of the queue.
    pub lane: bool,
}

impl NoticeSpec {
    /// The queue's notice of this entry for `event`, which it carries to be drawn.
    pub fn notice(&self, event: HudEvent) -> Notice<NoticeId, HudEvent> {
        Notice {
            key: self.id,
            payload: event,
            priority: self.priority,
            duration: self.duration,
            dedupe: Dedupe { window: self.dedupe },
            max_wait: self.max_wait,
        }
    }
}

/// An entry with the common values: timed, silent, still, folded within 2 s, waiting as long as it takes.
const fn entry(
    id: NoticeId,
    category: Category,
    priority: Priority,
    text: &'static str,
    icon: &'static str,
    duration: f32,
) -> NoticeSpec {
    NoticeSpec {
        id,
        category,
        priority,
        text: Some(text),
        secondary: None,
        icon,
        duration,
        dismissal: Dismissal::Timed,
        audio: None,
        haptic: None,
        dedupe: 2.0,
        max_wait: None,
        lane: false,
    }
}

/// The catalogue's entry for `event`; `None` for a phase change, which is the layout's.
pub fn spec(event: &HudEvent) -> Option<NoticeSpec> {
    use Category::*;
    use NoticeId as Id;
    use Priority::*;
    Some(match *event {
        HudEvent::PhaseChanged { .. } => return None,
        HudEvent::HeatUp { .. } => NoticeSpec {
            audio: Some(AudioCue::Escalation),
            haptic: Some(Haptic::Light),
            ..entry(Id::HeatUp, Pursuit, High, "hud-notify-heat-up", "heat", 3.0)
        },
        HudEvent::RoadblockReported => NoticeSpec {
            secondary: Some("hud-notify-roadblock-detail"),
            audio: Some(AudioCue::Hazard),
            haptic: Some(Haptic::Strong),
            dedupe: 10.0,
            ..entry(Id::Roadblock, Pursuit, Critical, "hud-notify-roadblock", "roadblock", 4.0)
        },
        HudEvent::HelicopterTracking => NoticeSpec {
            audio: Some(AudioCue::Escalation),
            dedupe: 10.0,
            ..entry(Id::Helicopter, Pursuit, High, "hud-notify-helicopter", "helicopter", 3.0)
        },
        HudEvent::ContactLost => NoticeSpec {
            secondary: Some("hud-notify-contact-lost-detail"),
            dismissal: Dismissal::WhilePhase(PursuitPhase::Search),
            audio: Some(AudioCue::ContactLost),
            dedupe: 5.0,
            ..entry(Id::ContactLost, Pursuit, Medium, "hud-notify-contact-lost", "eye-off", 4.0)
        },
        HudEvent::CooldownStarted => NoticeSpec {
            secondary: Some("hud-notify-cooldown-detail"),
            dismissal: Dismissal::WhilePhase(PursuitPhase::Cooldown),
            audio: Some(AudioCue::Cooldown),
            dedupe: 5.0,
            ..entry(Id::Cooldown, Pursuit, Medium, "hud-notify-cooldown", "timer", 4.0)
        },
        HudEvent::Escaped => NoticeSpec {
            audio: Some(AudioCue::Escaped),
            haptic: Some(Haptic::Light),
            dedupe: 10.0,
            ..entry(Id::Escaped, Pursuit, High, "hud-notify-escaped", "escaped", 4.0)
        },
        HudEvent::Busted => NoticeSpec {
            audio: Some(AudioCue::Busted),
            haptic: Some(Haptic::Strong),
            dedupe: 10.0,
            ..entry(Id::Busted, Pursuit, Critical, "hud-notify-busted", "busted", 4.0)
        },
        HudEvent::Checkpoint { .. } => NoticeSpec {
            audio: Some(AudioCue::Checkpoint),
            dedupe: 1.0,
            ..entry(Id::Checkpoint, Race, Medium, "hud-notify-checkpoint", "checkpoint", 2.0)
        },
        HudEvent::WrongWay => NoticeSpec {
            secondary: Some("hud-notify-wrong-way-detail"),
            audio: Some(AudioCue::MissedCheckpoint),
            haptic: Some(Haptic::Light),
            dedupe: 3.0,
            ..entry(Id::WrongWay, Race, Medium, "hud-notify-wrong-way", "wrong-way", 3.0)
        },
        HudEvent::Lap { .. } => NoticeSpec {
            audio: Some(AudioCue::Checkpoint),
            ..entry(Id::Lap, Race, Medium, "hud-notify-lap", "lap", 3.0)
        },
        // The place flickers when two cars run side by side: a repeat refreshes the one notice, and a stale one is
        // dropped rather than shown late.
        HudEvent::PositionChanged { .. } => NoticeSpec {
            dedupe: 3.0,
            max_wait: Some(1.0),
            ..entry(Id::Position, Race, Medium, "hud-notify-position", "position", 2.0)
        },
        HudEvent::NearMiss => NoticeSpec {
            dedupe: 1.5,
            max_wait: Some(1.0),
            lane: true,
            ..entry(Id::NearMiss, Driving, Low, "hud-notify-near-miss", "near-miss", 1.5)
        },
        HudEvent::SpeedTrap { passed: true, .. } => NoticeSpec {
            audio: Some(AudioCue::Checkpoint),
            lane: true,
            ..entry(Id::SpeedTrap, Career, Medium, "hud-notify-speed-trap", "speed-trap", 2.5)
        },
        HudEvent::SpeedTrap { passed: false, .. } => NoticeSpec {
            lane: true,
            ..entry(Id::SpeedTrapSlow, Career, Medium, "hud-notify-speed-trap-slow", "speed-trap", 2.5)
        },
        HudEvent::BountyMilestone { .. } => {
            NoticeSpec { dedupe: 5.0, ..entry(Id::Bounty, Career, Low, "hud-notify-bounty", "bounty", 3.0) }
        }
        HudEvent::BountyGained { .. } => {
            NoticeSpec { lane: true, ..entry(Id::BountyGain, Career, Low, "hud-feedback-bounty-gain", "bounty", 2.0) }
        }
        HudEvent::TirePunctured => NoticeSpec {
            audio: Some(AudioCue::CriticalDamage),
            haptic: Some(Haptic::Strong),
            dedupe: 5.0,
            ..entry(Id::TirePunctured, Vehicle, High, "hud-notify-tire", "tyre", 4.0)
        },
        HudEvent::HeavyDamage => NoticeSpec {
            audio: Some(AudioCue::CriticalDamage),
            haptic: Some(Haptic::Strong),
            dedupe: 10.0,
            ..entry(Id::HeavyDamage, Vehicle, High, "hud-notify-heavy-damage", "damage", 4.0)
        },
        HudEvent::VehicleCritical => NoticeSpec {
            audio: Some(AudioCue::CriticalDamage),
            haptic: Some(Haptic::Strong),
            dedupe: 10.0,
            ..entry(Id::VehicleCritical, Vehicle, High, "hud-notify-vehicle-critical", "warning", 5.0)
        },
        HudEvent::NitrousOn => NoticeSpec {
            text: None,
            audio: Some(AudioCue::Nitrous),
            haptic: Some(Haptic::Light),
            ..entry(Id::Nitrous, Driving, Low, "", "nitrous", 0.0)
        },
    })
}
