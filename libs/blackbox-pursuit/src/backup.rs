//! The wave budget of a chase and the backup that refills it (`docs/specs/ai-pursuit-cops.md` section 4,
//! `docs/specs/ai-pursuit-heat.md` section 2.1, `docs/specs/ai-pursuit.md` section 2).
//!
//! A chase starts needing `FullEngagementCopCount` cops evaded. Each cop that was ever within the engagement radius and
//! then leaves the chase (wrecked, left behind, recycled) counts as evaded, and the wave only asks for the cops still to
//! evade. When that falls to `NumCopsToTriggerBackup` the backup is called and counts down `BackupCallTimer` seconds
//! while the player is seen; when it arrives the budget is filled again.
//!
//! Nothing here reads a world or a clock: the caller says how many cops are engaged and how long a frame was.

/// What one update of the budget did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Nothing changed.
    None,
    /// The backup was called: the countdown started.
    Called,
    /// The countdown ended: the budget is full again.
    Arrived,
}

/// The cops-to-evade budget of one chase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    /// Cops that have to be evaded (`mNumCopsRequiredToEvade`): grows when more cops are engaged than remain.
    required: i32,
    /// Engaged cops that have left the chase (`mNumFullyEngagedCopsEvaded`).
    evaded: i32,
    /// The remaining count at or under which the backup is called (`NumCopsToTriggerBackup`).
    trigger: i32,
    /// Seconds until the backup arrives, while it is called.
    countdown: Option<f32>,
}

impl Budget {
    /// A fresh budget for a heat row.
    pub fn new(required: u32, trigger: u32) -> Self {
        Self { required: required as i32, evaded: 0, trigger: trigger as i32, countdown: None }
    }

    /// Fills the budget again from a row (`LockInPursuitAttribs`): the evaded count restarts and any call is over.
    pub fn refill(&mut self, required: u32, trigger: u32) {
        *self = Self::new(required, trigger);
    }

    /// An engaged cop left the chase.
    pub fn note_evaded(&mut self, cops: u32) {
        self.evaded = self.evaded.saturating_add(cops as i32);
    }

    /// Cops the wave may still ask for: `required - evaded`, never negative.
    pub fn allowed(&self) -> u32 {
        (self.required - self.evaded).max(0) as u32
    }

    /// Cops still to evade as the speech and the HUD read it: at least 1.
    pub fn remaining(&self) -> u32 {
        (self.required - self.evaded).max(1) as u32
    }

    /// Cops that have to be evaded in all.
    pub fn required(&self) -> u32 {
        self.required.max(0) as u32
    }

    /// Cops evaded so far.
    pub fn evaded(&self) -> u32 {
        self.evaded.max(0) as u32
    }

    /// Seconds until the backup arrives (`GetBackupETA`); `None` when it was not called.
    pub fn eta(&self) -> Option<f32> {
        self.countdown
    }

    /// Whether the backup has been called and has not arrived.
    pub fn is_called(&self) -> bool {
        self.countdown.is_some()
    }

    /// One update. `engaged` is how many non-support cops stand within the engagement radius now, `initial_chase` that the
    /// chase is in its initial phase (not cooling down), `call_timer` the row's `BackupCallTimer`. The count grows so that
    /// it never falls below one and never below the cops engaged; reaching the trigger calls the backup.
    pub fn update(&mut self, engaged: u32, initial_chase: bool, call_timer: f32) -> Event {
        let mut remaining = self.required - self.evaded;
        if remaining < 1 {
            remaining += 1;
            self.required += 1;
        }
        let surplus = engaged as i32 - remaining;
        if surplus > 0 {
            self.required += surplus;
        }
        let triggered = self.required != 0 && initial_chase && remaining <= self.trigger;
        if !triggered || self.countdown.is_some() {
            return Event::None;
        }
        self.countdown = Some(call_timer);
        Event::Called
    }

    /// Runs the countdown down while the player is in sight (`PS_BACKUP_REQUESTED`) and fills the budget when it ends.
    /// `required` and `trigger` are the row's values for the refill.
    pub fn tick(&mut self, dt: f32, in_sight: bool, required: u32, trigger: u32) -> Event {
        let Some(left) = self.countdown else { return Event::None };
        if !in_sight || !(dt > 0.0 && dt.is_finite()) {
            return Event::None;
        }
        let left = left - dt;
        if left >= 0.0 {
            self.countdown = Some(left);
            return Event::None;
        }
        self.refill(required, trigger);
        Event::Arrived
    }

    /// The cool-down took over: a backup that was called is dropped (the status is overwritten), and the next initial
    /// chase may call it again.
    pub fn cancel_call(&mut self) {
        self.countdown = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_budget_allows_the_full_engagement_count() {
        let budget = Budget::new(5, 2);
        assert_eq!((budget.allowed(), budget.remaining(), budget.required(), budget.evaded()), (5, 5, 5, 0));
        assert!(!budget.is_called());
    }

    #[test]
    fn each_cop_that_leaves_shrinks_what_the_wave_may_ask_for() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(2);
        assert_eq!(budget.allowed(), 3);
        budget.note_evaded(4);
        assert_eq!(budget.allowed(), 0, "never negative");
        assert_eq!(budget.remaining(), 1, "the readout never falls under one");
    }

    #[test]
    fn the_required_count_never_leaves_fewer_than_one_to_evade() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(5);
        budget.update(0, true, 180.0);
        assert_eq!(budget.required(), 6, "an empty budget asks for one more");
        assert_eq!(budget.allowed(), 1);
    }

    #[test]
    fn more_cops_engaged_than_remain_raise_the_required_count() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(3);
        budget.update(4, true, 180.0);
        assert_eq!(budget.required(), 5 + 2, "4 engaged against 2 remaining adds the difference");
        assert_eq!(budget.allowed(), 4);
    }

    #[test]
    fn reaching_the_trigger_calls_the_backup_once() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(2);
        assert_eq!(budget.update(3, true, 180.0), Event::None, "3 remain, the trigger is 2");
        budget.note_evaded(1);
        assert_eq!(budget.update(2, true, 180.0), Event::Called);
        assert_eq!(budget.eta(), Some(180.0));
        assert_eq!(budget.update(2, true, 180.0), Event::None, "called once");
    }

    #[test]
    fn the_backup_is_not_called_while_cooling_down() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(4);
        assert_eq!(budget.update(1, false, 180.0), Event::None);
        assert!(!budget.is_called());
        assert_eq!(budget.update(1, true, 180.0), Event::Called);
    }

    #[test]
    fn the_countdown_runs_only_while_the_player_is_in_sight_and_refills_the_budget() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(4);
        budget.update(1, true, 10.0);
        assert_eq!(budget.tick(4.0, false, 5, 2), Event::None, "out of sight the clock holds");
        assert_eq!(budget.eta(), Some(10.0));
        assert_eq!(budget.tick(4.0, true, 5, 2), Event::None);
        assert_eq!(budget.eta(), Some(6.0));
        assert_eq!(budget.tick(7.0, true, 5, 2), Event::Arrived);
        assert_eq!((budget.allowed(), budget.evaded()), (5, 0), "a full wave again");
        assert!(!budget.is_called());
    }

    #[test]
    fn a_cool_down_drops_the_call() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(4);
        budget.update(1, true, 90.0);
        assert!(budget.is_called());
        budget.cancel_call();
        assert!(!budget.is_called());
        assert_eq!(budget.tick(1.0, true, 5, 2), Event::None);
    }

    #[test]
    fn a_bad_step_changes_nothing() {
        let mut budget = Budget::new(5, 2);
        budget.note_evaded(4);
        budget.update(1, true, 10.0);
        let before = budget;
        assert_eq!(budget.tick(f32::NAN, true, 5, 2), Event::None);
        assert_eq!(budget.tick(-1.0, true, 5, 2), Event::None);
        assert_eq!(budget, before);
    }
}
