//! Per-unit pursuit sight on the car's own AI task. Spec: ai-pursuit-visibility-clocks.md.

use super::AiCar;
#[cfg(test)]
use crate::scenes::world::ai::cop::THINK_PERIOD;

#[derive(Debug, Clone)]
pub(super) struct Sight {
    active: bool,
    pending: std::collections::VecDeque<f32>,
    test_timer: f32,
    since_seen: f32,
}

impl Default for Sight {
    fn default() -> Self {
        Self { active: true, pending: Default::default(), test_timer: 0.0, since_seen: 99.0 }
    }
}

impl Sight {
    pub(super) fn without_target(&mut self) {
        self.pending.clear();
        self.test_timer = 0.25;
        self.since_seen = 0.25;
    }

    fn advance(&mut self, dt: f32) -> bool {
        self.test_timer += dt;
        self.since_seen += dt;
        if self.test_timer < 0.25 {
            return false;
        }
        self.test_timer -= 0.25;
        true
    }

    pub(super) fn hash_into(&self, hash: &mut crate::scenes::world::StateHasher) {
        hash.u32(self.pending.len() as u32);
        for &dt in &self.pending {
            hash.f32(dt);
        }
        hash.bool(self.active);
        hash.f32(self.test_timer);
        hash.f32(self.since_seen);
    }
}

impl AiCar {
    /// Original aggregation excludes inactive vehicles. Collision suspension is the world's activity boundary.
    pub(in crate::scenes::world::ai) fn set_sight_active(&mut self, active: bool) {
        if !self.sight.active && active {
            self.sight.without_target();
        }
        self.sight.active = active;
        if !active {
            self.sight.pending.clear();
        }
    }

    pub(in crate::scenes::world::ai) fn sight_active(&self) -> bool {
        self.sight.active
    }

    /// Called exactly when this car executes its existing staggered pursuit AI task.
    pub(super) fn queue_sight_task(&mut self) {
        self.sight.pending.push_back(self.think_delta);
    }

    /// Resolve queued tasks once. `None` means no task was pending; `Some` says whether its ray is due.
    pub(in crate::scenes::world::ai) fn next_sight_task(&mut self) -> Option<bool> {
        let dt = self.sight.pending.pop_front()?;
        Some(self.sight.advance(dt))
    }

    pub(in crate::scenes::world::ai) fn forget_sight_target(&mut self) {
        self.sight.without_target();
    }

    pub(in crate::scenes::world::ai) fn sight_age(&self) -> f32 {
        self.sight.since_seen
    }

    pub(super) fn saw_target(&mut self) {
        self.sight.since_seen = -0.25;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sight_clocks_use_each_tasks_scaled_elapsed_time() {
        let mut sight = Sight { since_seen: -0.25, ..Default::default() };
        for _ in 0..7 {
            assert!(!sight.advance(1.0 / 30.0));
        }
        assert!(sight.advance(1.0 / 30.0));
        assert!((sight.since_seen - 1.0 / 60.0).abs() < 1e-6);
        assert!((sight.test_timer - 1.0 / 60.0).abs() < 1e-6);
    }

    #[test]
    fn constructor_and_promoted_patrol_have_distinct_initial_test_phases() {
        let mut spawned = Sight::default();
        assert!(!spawned.advance(THINK_PERIOD));
        assert!(spawned.advance(THINK_PERIOD));
        let mut promoted = Sight::default();
        promoted.without_target();
        assert!(promoted.advance(THINK_PERIOD));
    }

    #[test]
    fn a_failed_query_does_not_clear_recent_sight_until_its_timer_advances_past_zero() {
        let mut sight = Sight { since_seen: -0.25, ..Default::default() };
        sight.advance(THINK_PERIOD);
        assert!(sight.since_seen <= 0.0);
        sight.advance(THINK_PERIOD);
        assert!(sight.since_seen > 0.0);
    }

    #[test]
    fn fractional_task_remainder_is_retained_across_visibility_queries() {
        let mut sight = Sight::default();
        let tests = (0..30).filter(|_| sight.advance(THINK_PERIOD)).count();
        assert_eq!(tests, 16);
        assert!(sight.test_timer >= 0.0 && sight.test_timer < 0.25);
    }
}
