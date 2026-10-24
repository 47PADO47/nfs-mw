//! The standing roadblocks of the world and the dispatch state that decides when the next one is built. A roadblock is
//! the cars that carry its group number, its props and the rules in `blackbox_pursuit::roadblock::standing`.

use std::rc::Rc;

use blackbox_pursuit::roadblock::request::Dispatcher;
use blackbox_pursuit::roadblock::standing::Standing;
use blackbox_pursuit::spikes::Strip;
use glam::Mat4;

use super::props::{Kit, Physics};

#[cfg(test)]
mod pending_tests {
    use super::*;

    #[test]
    fn blocked_heavy_placement_expires_at_its_selected_duration() {
        let mut blocks =
            Roadblocks { heavy: Some(HeavyRequest { big_suv: 80.0, remaining: 20.0 }), ..Default::default() };
        blocks.age_heavy(20.0);
        assert!(blocks.has_pending_support(), "native request expires strictly below zero");
        blocks.age_heavy(0.01);
        assert!(!blocks.has_pending_support());
    }
}

/// One roadblock that stands (`AIRoadBlock`). Its cars are the world's cars that carry `id` as their group.
pub(in crate::scenes::world::ai) struct Roadblock {
    pub id: u32,
    pub standing: Standing,
    /// Spike strips: the tire test's boxes and the models' frames (render space), in the same order.
    pub strips: Vec<Strip>,
    pub strip_frames: Vec<Mat4>,
    /// The sawhorses in the prop world.
    pub sawhorses: Vec<u32>,
    /// The sawhorses live while this handle does: releasing the roadblock drops it.
    pub owner: Rc<()>,
    /// Cars the cheat rule took into the chase (`mNumRBCopsAdded`): at most one.
    pub recruited: u32,
}

/// What the chase's roadblocks added up to, for the statistics (`NotifyRoadblockDeployed`, `NotifyRoadblockDodged`...).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub deployed: u32,
    pub dodged: u32,
    /// Props deployed: sawhorses and spike strips alike (`NotifySpikeStripDeployed` counts every prop).
    pub props_deployed: u32,
    pub spike_strips_dodged: u32,
    pub recruited: u32,
}

/// A heavy-support roadblock the chase asked for, waiting for the world to find a place for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct HeavyRequest {
    /// Big SUV chance of the option, percent.
    pub big_suv: f32,
    /// Pending support duration, seconds; expiry is independent of placement eligibility.
    pub remaining: f32,
}

/// All the roadblocks of the world, the request lottery and the loaded props.
#[derive(Default)]
pub struct Roadblocks {
    pub(super) list: Vec<Roadblock>,
    pub(super) next_id: u32,
    pub(super) dispatcher: Dispatcher,
    /// Seconds since the manager's last run (one per two simulation ticks).
    pub(super) clock: f32,
    pub(super) heavy: Option<HeavyRequest>,
    pub(super) stats: Stats,
    /// A roadblock car is within 60 m of the player and the player is not hidden: the cops see the player.
    pub(super) sees_player: bool,
    pub(super) kit: Option<Kit>,
    pub(super) physics: Option<Physics>,
    /// The props' models were asked for, and whether the attempt failed (not asked again).
    pub(super) asked: bool,
    pub(super) failed: bool,
}

impl Roadblocks {
    pub(in crate::scenes::world::ai) fn has_pending_support(&self) -> bool {
        self.heavy.is_some()
    }

    pub(super) fn age_heavy(&mut self, dt: f32) {
        if let Some(request) = self.heavy.as_mut() {
            request.remaining -= dt;
            if request.remaining < 0.0 {
                self.heavy = None;
            }
        }
    }

    /// A chase starts: the lottery, the request and the statistics begin again. Standing roadblocks stay.
    pub fn begin_chase(&mut self) {
        self.end_chase();
        self.stats = Stats::default();
    }

    /// A chase ends: the lottery and the request stop. The statistics stay until the next chase.
    pub fn end_chase(&mut self) {
        self.dispatcher.reset();
        self.heavy = None;
        self.clock = 0.0;
        self.sees_player = false;
    }

    /// A standing roadblock exists (the pursuit has one attached).
    pub fn any(&self) -> bool {
        !self.list.is_empty()
    }

    /// How many roadblocks stand.
    pub fn count(&self) -> usize {
        self.list.len()
    }

    /// Where the standing roadblocks are, in render space (`ROADBLOCK` map icons and the challenge series).
    pub fn sites(&self) -> Vec<[f32; 2]> {
        self.list
            .iter()
            .map(|rb| {
                let at = crate::scenes::world::space::to_render(rb.standing.centre.to_array());
                [at[0], at[1]]
            })
            .collect()
    }

    /// How many spike strips lie across the standing roadblocks.
    pub fn strip_count(&self) -> usize {
        self.list.iter().map(|rb| rb.strips.len()).sum()
    }

    /// The props' models are uploaded.
    pub fn has_models(&self) -> bool {
        self.kit.is_some()
    }

    /// A roadblock request is latched: the cop manager spawns no pursuit cops meanwhile.
    pub fn dispatcher_latched(&self) -> bool {
        self.dispatcher.is_latched()
    }

    /// The statistics of the chase.
    pub fn stats(&self) -> Stats {
        self.stats
    }

    /// The pursuit holds the follow formation: a roadblock is up, not passed and untouched.
    pub fn forces_follow(&self) -> bool {
        self.list.iter().any(|rb| rb.standing.forces_follow())
    }

    /// Every spike strip of every standing roadblock.
    pub fn strips(&self) -> Vec<Strip> {
        self.list.iter().flat_map(|rb| rb.strips.iter().copied()).collect()
    }

    /// The models of the props are wanted and not asked for yet.
    pub fn wants_models(&self) -> bool {
        self.kit.is_none() && !self.asked && !self.failed
    }

    /// Takes the roadblocks away with their props. The dispatch state is left alone.
    pub fn clear(&mut self) {
        self.list.clear();
        self.sees_player = false;
    }

    /// A prop of the prop world is a sawhorse of a standing roadblock.
    pub fn owns_sawhorse(&self, id: u32) -> bool {
        self.list.iter().any(|block| block.sawhorses.contains(&id))
    }

    /// A roadblock car is close enough to see the player (`kRoadBlockLOS`), as of the last frame.
    pub fn sees_player(&self) -> bool {
        self.sees_player
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_world_has_no_roadblock_and_wants_its_models() {
        let roadblocks = Roadblocks::default();
        assert!(!roadblocks.any());
        assert!(roadblocks.wants_models());
        assert!(!roadblocks.forces_follow());
    }

    #[test]
    fn a_failed_load_is_not_asked_for_again() {
        let roadblocks = Roadblocks { failed: true, ..Roadblocks::default() };
        assert!(!roadblocks.wants_models());
    }
}
