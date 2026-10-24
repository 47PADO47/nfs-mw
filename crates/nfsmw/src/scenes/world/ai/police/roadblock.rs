//! The police side of the roadblocks: what the chase tells the world's roadblock director and what it takes back. The
//! roadblocks themselves are built and tracked in `ai/roadblock`; the chase keeps the formation rule (a roadblock that
//! stands, is whole and has not been passed holds the follow formation), the heavy request's active time and the cost
//! to state. Child of `police`.

use super::Police;

/// The chase's roadblock state.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct State {
    /// A roadblock is up, not passed and untouched: the follow formation holds.
    pub follow: bool,
    /// The duration of the heavy roadblock option that was rolled, seconds.
    pub duration: f32,
    /// Seconds the built heavy roadblock request stays active; no other ground support is rolled meanwhile.
    pub active_secs: f32,
}

impl Police {
    pub(in crate::scenes::world::ai) fn roadblock_request_duration(&self) -> f32 {
        self.roadblock.duration
    }

    /// Whether a roadblock was asked for since the last call (the world places it).
    pub fn take_roadblock_request(&mut self) -> bool {
        std::mem::take(&mut self.roadblock_requested)
    }

    /// The world placed the roadblock: the chase does not roll another heavy roadblock.
    pub fn roadblock_placed(&mut self) {
        self.roadblock_standing = true;
        self.on_roadblock();
        self.roadblock_added();
    }

    /// A roadblock of this chase stands.
    pub fn roadblock_standing(&self) -> bool {
        self.roadblock_standing
    }

    /// The world reports whether a roadblock stands (the pursuit has one attached): the heavy roll skips its options then.
    pub fn set_roadblock_standing(&mut self, standing: bool) {
        self.roadblock_standing = standing;
    }

    /// The world reports whether a standing roadblock holds the follow formation.
    pub fn set_roadblock_follow(&mut self, follow: bool) {
        self.roadblock.follow = follow;
    }

    /// `count` props (sawhorses and spike strips) were deployed with a roadblock: their cost to state.
    pub fn roadblock_props(&mut self, count: u32) {
        self.on_roadblock_props(count);
    }

    /// The heavy roadblock request was met: it is active for the option's duration (`mSupportRequestStatus` ACTIVE).
    pub fn heavy_roadblock_built(&mut self) {
        self.roadblock.active_secs = self.roadblock.duration;
    }

    /// A roadblock was added to the chase: the current formation ends unless it is FOLLOW or its finisher runs
    /// (`AIPursuit::AddRoadBlock`).
    pub(super) fn roadblock_added(&mut self) {
        let following = self.last_formation == Some(blackbox_pursuit::formation::Formation::Follow);
        if following || self.finisher.is_running() {
            return;
        }
        self.formations.heat_changed();
    }

    /// The heavy roll's active time runs down; true while the request is active and no roll may be made.
    pub(super) fn heavy_roadblock_active(&mut self, dt: f32) -> bool {
        let active = self.roadblock.active_secs > 0.0;
        self.roadblock.active_secs = (self.roadblock.active_secs - dt).max(0.0);
        active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_built_heavy_roadblock_does_not_pause_the_next_support_check_clock() {
        let mut police = Police::default();
        police.begin(5);
        police.pursuit_time = 1000.0;
        police.roadblock.duration = 20.0;
        police.heavy_roadblock_built();
        police.tick_support(20.0, true, 5, [0.99; 2]);
        assert!(police.support_timer < 0.0);
        police.tick_support(0.1, true, 5, [0.99; 2]);
        assert_eq!(police.support_timer, super::super::SUPPORT_CHECK);
    }
}
