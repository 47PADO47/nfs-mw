//! The shared cell the Remastered cluster, minimap, radar and pursuit timer read their state from. The Custom HUD reads
//! its own, richer cell (`feed.rs`); this one stays as Remastered knows it.
//!
//! The same producer rule as `radio_feed.rs`: the HUD plugin copies the state in once per frame, and the components only
//! read it. The copy is plain data, so the cell is `Send + Sync`, as `Theme` requires.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::hud::{HudState, PursuitHud};
use crate::ui::present::Screen;

/// One frame's copy of the drive state.
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::ui::theme) struct DriveFeed {
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
    /// The car's retained heat (`HudState::vehicle_heat`): 1.0 and up, shown as a whole multiplier.
    pub heat: f32,
    /// The cop cars the chase has called, and how many of them are damaged now (`HudState::cops_engaged`, `cops_damaged`).
    pub cops_engaged: u32,
    pub cops_damaged: u32,
    /// The nearest cop in the detector's cone and range, if any (`HudState::radar`).
    pub radar: Option<crate::hud::RadarHud>,
    /// A race's own map is on the minimap: its route is a white band that a darkening tint would turn grey.
    pub route_map: bool,
}

/// The shared cell. Clones share one feed.
#[derive(Clone, Debug, Default)]
pub struct DriveFeedCell {
    feed: Arc<Mutex<DriveFeed>>,
}

impl DriveFeedCell {
    /// An empty cell: the components read zeros until the plugin sets it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies this frame's drive state into the cell. Replaces the previous frame's copy. The screen is not read: the
    /// Remastered components place themselves from the canvas.
    pub(crate) fn set(&self, state: &HudState, _screen: Screen) {
        let scale = state.scale_rpm();
        let feed = DriveFeed {
            speed: state.display_speed(),
            mph: state.use_mph,
            rpm: state.rpm_fraction(),
            red_line: (state.red_line / scale).clamp(0.0, 1.0),
            gear: state.gear,
            scale_rpm: scale,
            shift_light: state.shift_light,
            has_nos: state.has_nos,
            nos: state.nos,
            has_turbo: state.has_turbo,
            boost_psi: state.boost_psi,
            speedbreaker: state.speedbreaker,
            speedbreaker_active: state.speedbreaker_active,
            pursuit: state.pursuit,
            heat: state.vehicle_heat,
            cops_engaged: state.cops_engaged,
            cops_damaged: state.cops_damaged,
            radar: state.radar,
            route_map: state.map_source.event().is_some(),
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
