//! The HUD's notification manager: the frame's [`HudEvent`]s become notices of the catalogue (`catalog.rs`) in a
//! priority queue (`blackbox_ui::notify`), which decides what is on the screen. The HUD plugin feeds it once a frame
//! (`plugin.rs`), and the Custom HUD draws what is visible.
//!
//! - `catalog`: what each kind of event tells the player, and how.
//!
//! The queue is fed from the state's event list once per frame: a scene that hands the same frame over again (a paused
//! world) adds nothing, and an identical game state makes no events in the first place. The game is intense while a
//! chase is active or a race is under way: the Low and Background notices wait then.

mod catalog;
#[cfg(test)]
mod tests;

use bevy_ecs::prelude::Resource;
use blackbox_ui::NotifyQueue;
use blackbox_ui::notify::Priority;

pub use catalog::NoticeId;
use catalog::{Dismissal, spec};

use super::{HudEvent, HudState, PursuitPhase};

/// Which notifications the player wants: the `[hud] notifications` setting. Until that setting is read, the HUD passes
/// [`NotifyLevel::All`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)] // the [hud] notifications setting picks Important and Off; it is not wired yet
pub enum NotifyLevel {
    #[default]
    All,
    /// Only the Critical and High ones.
    Important,
    Off,
}

impl NotifyLevel {
    /// Whether a notice of `priority` is shown at this level.
    fn admits(self, priority: Priority) -> bool {
        match self {
            NotifyLevel::All => true,
            NotifyLevel::Important => priority <= Priority::High,
            NotifyLevel::Off => false,
        }
    }
}

/// The HUD's notifications: the queue and the last frame of events it took.
#[derive(Resource, Default)]
pub struct HudNotifications {
    queue: NotifyQueue<NoticeId, HudEvent>,
    /// The `HudState::events_frame` of the events last taken.
    taken: Option<u64>,
}

impl HudNotifications {
    /// Take the frame's events from `state` into the queue, at the player's `level`. A frame already taken adds
    /// nothing. A phase change takes away the notices that last only while the chase was in the phase it left.
    pub fn take_events(&mut self, state: &HudState, level: NotifyLevel) {
        if self.taken == Some(state.events_frame) {
            return;
        }
        self.taken = Some(state.events_frame);
        for event in &state.events {
            if let HudEvent::PhaseChanged { from, .. } = *event {
                self.end_phase(from);
            }
            let Some(entry) = spec(event) else { continue };
            if entry.text.is_none() || !level.admits(entry.priority) {
                continue;
            }
            self.queue.push(entry.notice(*event));
        }
    }

    /// Advance the notices by `dt` seconds; while `intense`, the Low and Background ones wait.
    pub fn tick(&mut self, dt: f32, intense: bool) {
        self.queue.tick(dt, intense);
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
