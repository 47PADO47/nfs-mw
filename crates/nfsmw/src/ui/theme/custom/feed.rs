//! The one shared cell every Custom HUD component reads (ADR 0013, decision 2): the frame's [`HudState`], the
//! `[custom_hud]` options, the speed unit, the frame clock and the window.
//!
//! The same producer rule as `drive_feed.rs` and `radio_feed.rs`: the HUD plugin sets the cell once per frame
//! (`hud/plugin.rs`, `present`), and the components only read it. The HUD state is behind an `Arc`, so the plugin
//! clones it once per frame and a component's read is a reference count, not a copy. The cell is `Send + Sync`, as
//! `Theme` requires.
//!
//! The Remastered theme does not read this cell: it keeps `DriveFeedCell`.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::hud::HudState;
use crate::settings::hud::HudSettings;
use crate::settings::{SpeedUnits, Transmission};
use crate::ui::present::Screen;

use super::cluster::ClusterMemory;
use super::layout::CANVAS;

/// What the notification manager shows this frame. A placeholder until the notification queue is drawn (PR 8): the
/// field that holds it, [`CustomFeed::notifications`], is `None` until then.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotificationView {}

/// One frame of what the Custom HUD draws from.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomFeed {
    /// The frame's HUD state, shared: cloning the feed does not copy it.
    pub hud: Arc<HudState>,
    /// The `[custom_hud]` options (`Settings::hud_options`).
    pub settings: HudSettings,
    /// The unit the player reads speed in (`Settings::speed_units`).
    pub units: SpeedUnits,
    /// Who changes gear (`Settings::transmission`): the cluster's shift cue also shows for a manual box past the shift
    /// point.
    pub transmission: Transmission,
    /// Seconds since the game started, on the frame clock. Animation runs on this, never on frame counts.
    pub elapsed_s: f32,
    /// The frame's length, in seconds, for the springs that integrate over time.
    pub delta_s: f32,
    /// The window, in points: the layout's content box comes from it (`layout.rs`).
    pub screen: Screen,
    /// The notifications on screen. `None` until the notification manager fills it (PR 8).
    pub notifications: Option<NotificationView>,
}

impl Default for CustomFeed {
    /// The empty feed the components read before the first frame: a default HUD state on the FEng canvas.
    fn default() -> Self {
        Self {
            hud: Arc::default(),
            settings: HudSettings::default(),
            units: SpeedUnits::default(),
            transmission: Transmission::default(),
            elapsed_s: 0.0,
            delta_s: 0.0,
            screen: CANVAS,
            notifications: None,
        }
    }
}

/// The shared cell. Clones share one feed, and one memory of the cluster's animation.
#[derive(Clone, Debug, Default)]
pub struct CustomFeedCell {
    feed: Arc<Mutex<CustomFeed>>,
    cluster: ClusterMemory,
}

impl CustomFeedCell {
    /// An empty cell: the components read [`CustomFeed::default`] until the plugin sets it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores this frame's feed, replacing the previous frame's.
    pub fn set(&self, feed: CustomFeed) {
        *self.lock() = feed;
    }

    /// This frame's feed. The HUD state is shared, so the copy is cheap.
    pub fn get(&self) -> CustomFeed {
        self.lock().clone()
    }

    /// What the cluster keeps between frames (its springs and pulses). The theme builds a new cluster each frame, so
    /// the memory lives with the feed it is stepped from.
    pub fn cluster(&self) -> &ClusterMemory {
        &self.cluster
    }

    /// The feed lock. A poisoned lock still holds a whole feed (each write replaces all of it), so the poison is
    /// ignored.
    fn lock(&self) -> MutexGuard<'_, CustomFeed> {
        self.feed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_cell_reads_the_default_feed() {
        let feed = CustomFeedCell::new().get();
        assert_eq!(feed, CustomFeed::default());
        assert!(feed.notifications.is_none(), "no notification view before PR 8");
    }

    #[test]
    fn set_replaces_the_feed_and_every_clone_reads_it() {
        let cell = CustomFeedCell::new();
        let reader = cell.clone();
        let hud = Arc::new(HudState { speed: 30.0, ..HudState::default() });
        let feed = CustomFeed {
            hud: Arc::clone(&hud),
            units: SpeedUnits::Imperial,
            elapsed_s: 12.5,
            delta_s: 1.0 / 60.0,
            screen: Screen { width: 2560.0, height: 1080.0, pixels_per_point: 1.0 },
            ..CustomFeed::default()
        };
        cell.set(feed.clone());
        let read = reader.get();
        assert_eq!(read, feed);
        assert!(Arc::ptr_eq(&read.hud, &hud), "the HUD state is shared, not copied");
        cell.set(CustomFeed::default());
        assert_eq!(reader.get(), CustomFeed::default(), "the next set replaces the whole feed");
    }
}
