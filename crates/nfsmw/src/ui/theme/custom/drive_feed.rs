//! The shared cell the Custom speedometer and tachometer read their state from.
//!
//! The same producer rule as `radio_feed.rs`: the HUD plugin copies the state in once per frame, and the components only
//! read it. The copy is plain data, so the cell is `Send + Sync`, as `Theme` requires.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::hud::{HudState, PursuitHud};
use crate::ui::present::Screen;

use super::drive_layout::Corner;

/// One frame's copy of the drive state.
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::ui::theme) struct DriveFeed {
    /// The window's bottom-right corner, where the cluster is laid out from (`drive_layout`).
    pub corner: Corner,
    /// The speed in the unit shown, as whole units (`HudState::display_speed`).
    pub speed: u32,
    /// Whether the speed is in miles per hour (otherwise kilometres per hour).
    pub mph: bool,
    /// 0 to 1 along the tachometer scale (`HudState::rpm_fraction`).
    pub rpm: f32,
    /// 0 to 1 along the tachometer scale: where the red line starts.
    pub red_line: f32,
    /// -1 reverse, 0 neutral, 1 and up forward.
    pub gear: i32,
    /// The tachometer's top, in rpm (`HudState::scale_rpm`). The Remastered dial numbers its ticks against it.
    pub scale_rpm: f32,
    /// The shift light is on: the gearbox wants the next gear.
    pub shift_light: bool,
    /// The car has nitrous (`HudState::has_nos`), and how much is left, 0 to 1 (`HudState::nos`).
    pub has_nos: bool,
    pub nos: f32,
    /// The car has forced induction (`HudState::has_turbo`), and its boost in psi (`HudState::boost_psi`).
    pub has_turbo: bool,
    pub boost_psi: f32,
    /// The speed breaker meter, 0 to 1 (`HudState::speedbreaker`), and whether the mode is active.
    pub speedbreaker: f32,
    pub speedbreaker_active: bool,
    /// The chase the pursuit board shows, or `None` while no cop chases the player (`HudState::pursuit`).
    pub pursuit: Option<PursuitHud>,
}

/// The shared cell. Clones share one feed.
#[derive(Clone, Debug, Default)]
pub struct DriveFeedCell {
    feed: Arc<Mutex<DriveFeed>>,
}

impl DriveFeedCell {
    /// An empty cell: the drive components read zeros until the plugin sets it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies this frame's drive state into the cell. Replaces the previous frame's copy.
    pub(crate) fn set(&self, state: &HudState, screen: Screen) {
        let scale = state.scale_rpm();
        let feed = DriveFeed {
            speed: state.display_speed(),
            mph: state.use_mph,
            rpm: state.rpm_fraction(),
            red_line: (state.red_line / scale).clamp(0.0, 1.0),
            gear: state.gear,
            scale_rpm: scale,
            shift_light: state.shift_light,
            corner: Corner::of(screen),
            has_nos: state.has_nos,
            nos: state.nos,
            has_turbo: state.has_turbo,
            boost_psi: state.boost_psi,
            speedbreaker: state.speedbreaker,
            speedbreaker_active: state.speedbreaker_active,
            pursuit: state.pursuit,
        };
        *self.lock() = feed;
    }

    /// This frame's copy, as the components read it.
    pub(in crate::ui::theme) fn snapshot(&self) -> DriveFeed {
        self.lock().clone()
    }

    /// The feed lock. A poisoned lock still holds a whole feed (each write replaces all of it), so the poison is ignored.
    fn lock(&self) -> MutexGuard<'_, DriveFeed> {
        self.feed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
