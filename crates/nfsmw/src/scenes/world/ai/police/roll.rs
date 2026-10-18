//! The support roll of a chase (spec `ai-pursuit-formations.md` section 6): the check every `SUPPORT_CHECK` seconds after the
//! level's minimum delay, while the cops see the player. The priority check of the leader options runs once per heat, then
//! the base roll of the heavy and leader options. A heavy roadblock is recorded for the world to place; a ram or leader
//! request waits for the chase to take it ([`Police::take_support_request`]). Child of `police`.

use blackbox_pursuit::support::{Option_, Request, Strategy};

use super::super::support;
use super::{Police, SUPPORT_CHECK};

impl Police {
    /// The support check: after the minimum delay, the roll of this check, then the request it made.
    pub(super) fn tick_support(&mut self, dt: f32, seen: bool, level: u32, random: f32) {
        let before = self.pursuit_time;
        self.pursuit_time += dt;
        self.tick_rep(before, dt, seen, level);
        self.support_timer -= dt;
        if self.support_timer > 0.0 || self.pursuit_time < support::minimum_delay(level) {
            return;
        }
        self.support_timer = SUPPORT_CHECK;
        if !seen {
            return;
        }
        let Some(request) = self.support_roll(level, random) else { return };
        match request.strategy {
            Strategy::HeavyRoadblock => self.roadblock_requested = true,
            strategy => {
                log::info!("support: {strategy:?} rolled for the chase");
                self.support_requested = Some(request);
            }
        }
    }

    /// The roll: the priority check of the leader options (once per heat, the first check after a leader option's
    /// priority time), then the base roll over the heavy options (none while a roadblock stands) and the leader options.
    /// One uniform number serves both rolls; the binary draws one for each (a deviation, see the port status).
    fn support_roll(&mut self, level: u32, random: f32) -> Option<Request> {
        if self.priority_heat != level {
            self.priority_heat = level;
            self.priority_done = false;
        }
        let leaders = support::leader_options(level);
        if !self.priority_done && leaders.iter().any(|&(_, _, time)| time < self.pursuit_time) {
            self.priority_done = true;
            if let Some(request) = blackbox_pursuit::support::priority(&leaders, self.pursuit_time, random) {
                return Some(request);
            }
        }
        let leader_options: Vec<Option_> = leaders.iter().map(|&(option, _, _)| option).collect();
        blackbox_pursuit::support::roll(
            &support::heavy_options(level),
            &leader_options,
            random,
            self.roadblock_standing,
        )
    }

    /// The ram or leader request the roll made since the last call, for the chase to take (`TrafficWorld::ask_support`).
    pub fn take_support_request(&mut self) -> Option<Request> {
        self.support_requested.take()
    }
}
