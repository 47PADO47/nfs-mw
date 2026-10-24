//! The HUD's notification manager: the frame's [`HudEvent`]s become notices of the catalogue (`catalog.rs`) in a
//! priority queue (`blackbox_ui::notify`), which decides what is on the screen. The HUD plugin feeds it once a frame
//! (`plugin.rs`), and the Custom HUD draws what is visible.
//!
//! - `catalog`: what each kind of event tells the player, and how.
//! - `feedback`: the feedback lane's chips (near misses, speed traps, bounty gains) and the nitrous pulse.
//! - `signals`: the sounds and rumbles the notices ask for, once each.
//! - `view`: what the Custom HUD draws of the notices and the lane this frame.
//!
//! The queue is fed from the state's event list once per frame: a scene that hands the same frame over again (a paused
//! world) adds nothing, and an identical game state makes no events in the first place. The game is intense while a
//! chase is active or a race is under way: the Low and Background notices wait then.

mod catalog;
mod feedback;
mod signals;
#[cfg(test)]
mod tests;
mod view;

use bevy_ecs::prelude::Resource;
use blackbox_ui::NotifyQueue;
use blackbox_ui::notify::Priority;

pub use catalog::{AudioCue, Category, Haptic, NoticeId};
use catalog::{Dismissal, spec};
use feedback::FeedbackLane;
pub use feedback::{Chip, ChipKind};
use signals::Signals;

use super::{HudEvent, HudState, PursuitPhase};
use crate::settings::hud::NotificationLevel;

/// Whether the player's `[custom_hud] notifications` level shows a notice of `priority`: `all` shows everything,
/// `important` the Critical, High and Medium ones, and `off` the Critical ones only, which are never filtered out.
fn admits(level: NotificationLevel, priority: Priority) -> bool {
    match level {
        NotificationLevel::All => true,
        NotificationLevel::Important => priority <= Priority::Medium,
        NotificationLevel::Off => priority == Priority::Critical,
    }
}

/// The HUD's notifications: the queue, the feedback lane, the sounds and rumbles they ask for, and the last frame of
/// events they took.
#[derive(Resource, Default)]
pub struct HudNotifications {
    queue: NotifyQueue<NoticeId, HudEvent>,
    lane: FeedbackLane,
    signals: Signals,
    /// The notices visible after the last tick, by key and `started`: a notice not among them has come up since.
    shown: Vec<(NoticeId, f32)>,
    /// The `HudState::events_frame` of the events last taken.
    taken: Option<u64>,
}

impl HudNotifications {
    /// Take the frame's events from `state` into the queue, at the player's `level`. A frame already taken adds
    /// nothing. A phase change takes away the notices that last only while the chase was in the phase it left. A kind
    /// with no notice raises its sound and rumble now, and a lane kind goes to the feedback lane (unless the level is
    /// `off`).
    pub fn take_events(&mut self, state: &HudState, level: NotificationLevel) {
        if self.taken == Some(state.events_frame) {
            return;
        }
        self.taken = Some(state.events_frame);
        for event in &state.events {
            if let HudEvent::PhaseChanged { from, .. } = *event {
                self.end_phase(from);
            }
            let Some(entry) = spec(event) else { continue };
            if entry.text.is_none() {
                self.lane.push(*event);
                self.signals.raise(&entry, self.queue.clock());
                continue;
            }
            if entry.lane {
                if level == NotificationLevel::Off {
                    continue;
                }
                self.lane.push(*event);
                self.signals.raise(&entry, self.queue.clock());
                continue;
            }
            if !admits(level, entry.priority) {
                continue;
            }
            self.queue.push(entry.notice(*event));
        }
    }

    /// Advance the notices and the lane by `dt` seconds; while `intense`, the Low and Background notices wait. The
    /// notices that came up since the last tick raise their sound and rumble.
    pub fn tick(&mut self, dt: f32, intense: bool) {
        self.queue.tick(dt, intense);
        self.lane.tick(dt);
        self.note_shown();
    }

    /// The sounds the notices asked for since the last call.
    pub fn take_cues(&mut self) -> Vec<AudioCue> {
        self.signals.take_cues()
    }

    /// The pad rumbles the notices asked for since the last call.
    pub fn take_haptics(&mut self) -> Vec<Haptic> {
        self.signals.take_haptics()
    }

    /// Raise the sound and rumble of each visible notice that was not visible (with the same `started`) after the
    /// last tick: a notice that came up, not one refreshed by a repeat.
    fn note_shown(&mut self) {
        let clock = self.queue.clock();
        let mut shown = Vec::with_capacity(self.queue.visible().len());
        for active in self.queue.visible() {
            shown.push((active.key, active.started));
            if self.shown.contains(&(active.key, active.started)) {
                continue;
            }
            let Some(entry) = spec(&active.payload) else { continue };
            self.signals.raise(&entry, clock);
        }
        self.shown = shown;
    }

    /// Take the oldest visible notice away (the dismiss action). Returns whether there was one.
    pub fn dismiss_oldest(&mut self) -> bool {
        let Some(key) = self.queue.visible().first().map(|active| active.key) else { return false };
        self.queue.dismiss(&key)
    }

    /// Take away the notices that last only while the chase is in `phase`.
    fn end_phase(&mut self, phase: PursuitPhase) {
        let ended: Vec<NoticeId> = self
            .queue
            .visible()
            .iter()
            .filter(|active| spec(&active.payload).is_some_and(|entry| entry.dismissal == Dismissal::WhilePhase(phase)))
            .map(|active| active.key)
            .collect();
        for key in ended {
            self.queue.dismiss(&key);
        }
    }
}

/// Whether the game is intense for the notifications: a chase with the player in sight, or a race past its countdown.
pub fn intense(state: &HudState) -> bool {
    state.pursuit_phase == PursuitPhase::Active || state.race.is_some_and(|race| race.count.is_none())
}
