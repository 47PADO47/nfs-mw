//! The world's side of the HUD's events (`hud.rs`): a frame starts with an empty list, the game's own sources push into
//! it while the world updates, and the frame ends with the chase's notices and the diff of the frame's snapshot.

use super::hud::FeedSnapshot;
use crate::hud::{ConditionWarning, HEAVY_DAMAGE, PursuitPhase};
use crate::scenes::world::WorldScene;

impl WorldScene {
    /// Start the frame's HUD events: the last frame's are gone.
    pub(in crate::scenes::world) fn begin_hud_feed(&mut self) {
        self.hud_feed.begin_frame();
    }

    /// End the frame's HUD events, `dt` seconds after the last frame: the chase's notices of the frame, then the
    /// changes since the last frame's snapshot.
    pub(in crate::scenes::world) fn end_hud_feed(&mut self, dt: f32) {
        let notices = self.traffic.as_mut().map(|traffic| traffic.take_pursuit_notices()).unwrap_or_default();
        self.hud_feed.notices(&notices);
        let snapshot = self.feed_snapshot();
        self.hud_feed.observe(snapshot, dt);
    }

    /// What the HUD's events are diffed from this frame.
    fn feed_snapshot(&self) -> FeedSnapshot {
        let drive = self.players.lead().and_then(|drive| drive.hud_state());
        let condition = drive.as_ref().and_then(|state| state.condition);
        let traffic = self.traffic.as_ref();
        FeedSnapshot {
            phase: traffic.map_or(PursuitPhase::None, |traffic| traffic.pursuit_phase()),
            roadblock_reported: traffic.is_some_and(|traffic| traffic.roadblock_reported()),
            helicopter_tracking: traffic.is_some_and(|traffic| traffic.helicopter_tracking()),
            place: self.race_hud().map(|race| race.place),
            near_misses: traffic.map_or(0, |traffic| traffic.near_misses()),
            bounty: self.career.hud.bounty(),
            damaged_tires: condition.map_or(0, |c| c.damaged_tires()),
            heavy_damage: condition.is_some_and(|c| c.cosmetic.is_some_and(|level| level >= HEAVY_DAMAGE)),
            critical: condition.and_then(|c| c.warning()) == Some(ConditionWarning::Critical),
            nos_burning: drive.is_some_and(|state| state.telemetry.nos_burning),
        }
    }
}
