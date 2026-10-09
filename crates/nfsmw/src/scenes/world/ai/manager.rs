//! The traffic manager's choices: how fast cars may appear, which type comes next and when an unseen car
//! is recycled. Pure numbers, so it can be tested without a world.
//! Spec: `docs/specs/ai-traffic-spawning.md` (§2, §5, §8).

use blackbox_roads::RandomSource;

/// The density tables, indexed by `density` over `[0, 1]` in 11 equal steps (§2).
const SPAWN_RATE: [f32; 11] = [0.0, 0.05, 0.1, 0.125, 0.2, 0.4, 0.6, 1.0, 3.0, 5.0, 8.0];
const OFFSCREEN_DISTANCE: [f32; 11] = [130.0, 120.0, 110.0, 100.0, 90.0, 80.0, 70.0, 60.0, 55.0, 45.0, 40.0];
const OFFSCREEN_TIME: [f32; 11] = [12.0, 10.0, 9.0, 8.0, 7.0, 6.5, 6.0, 5.5, 5.0, 4.5, 4.0];
/// A pursuit takes a quarter of the traffic away.
pub const PURSUIT_DENSITY_SCALE: f32 = 0.75;
/// Type records the manager looks at.
const MAX_RECORDS: usize = 10;

fn table(values: &[f32; 11], density: f32) -> f32 {
    let at = density.clamp(0.0, 1.0) * 10.0;
    let low = (at.floor() as usize).min(10);
    let high = (low + 1).min(10);
    values[low] + (values[high] - values[low]) * (at - low as f32)
}

/// How much faster than real time the type timers run at `density`.
pub fn spawn_rate(density: f32) -> f32 {
    table(&SPAWN_RATE, density)
}

/// A car this far (metres) from the player and out of view for [`offscreen_time`] seconds is recycled.
pub fn offscreen_distance(density: f32) -> f32 {
    table(&OFFSCREEN_DISTANCE, density)
}

pub fn offscreen_time(density: f32) -> f32 {
    table(&OFFSCREEN_TIME, density)
}

/// One car type of a traffic pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub name: String,
    /// Density-weighted waiting time the type needs before it may be chosen again.
    pub rate: f32,
    /// At most this many active cars of the type (0: no limit).
    pub max_instances: u32,
    /// The type may take at most this share of the traffic slots (0: no limit).
    pub percent: u32,
}

/// What the manager knows of the cars on the road when it picks a type.
pub trait Census {
    /// Active traffic cars of the model `name`.
    fn active(&self, name: &str) -> u32;
    /// Vehicles that are not traffic cars (the player, cops).
    fn others(&self) -> u32;
}

/// The type timers and the round-robin cursor.
#[derive(Debug, Clone, Default)]
pub struct TypeTimers {
    timers: Vec<f32>,
    cursor: usize,
}

impl TypeTimers {
    /// Starts every timer at a random fraction of its rate (a random head start).
    pub fn reset(&mut self, rules: &[Rule], rng: &mut impl RandomSource) {
        self.timers = rules.iter().take(MAX_RECORDS).map(|r| r.rate * rng.next_f32()).collect();
        self.cursor = 0;
    }

    /// Whether the timers have been started for a pattern.
    pub fn started(&self) -> bool {
        !self.timers.is_empty()
    }

    pub fn advance(&mut self, dt: f32, density: f32) {
        for t in &mut self.timers {
            *t += dt * spawn_rate(density);
        }
    }

    /// The index of the next type to spawn, looking at each record at most once starting at the cursor.
    /// `slots` is how many traffic cars the world has room for.
    pub fn next_type(&mut self, rules: &[Rule], slots: u32, census: &impl Census) -> Option<usize> {
        let n = rules.len().min(MAX_RECORDS).min(self.timers.len());
        if n == 0 {
            return None;
        }
        for _ in 0..n {
            let at = self.cursor % n;
            self.cursor = (self.cursor + 1) % n;
            let rule = &rules[at];
            let count = census.active(&rule.name);
            let free = slots.saturating_sub(census.others());
            let eligible = rule.rate > 0.0 && self.timers[at] > rule.rate;
            let within_max = rule.max_instances == 0 || count < rule.max_instances;
            let within_share = rule.percent == 0 || count < (free * rule.percent / 100).max(1);
            if eligible && within_max && within_share {
                return Some(at);
            }
        }
        None
    }

    /// A car of `name` was spawned: every record of that model starts waiting again.
    pub fn spawned(&mut self, rules: &[Rule], name: &str) {
        for (timer, rule) in self.timers.iter_mut().zip(rules) {
            if rule.name == name {
                *timer = 0.0;
            }
        }
    }
}

/// How much traffic is asked for: full in free roam, a quarter less during a pursuit, none without any.
pub fn density(wanted: bool, pursuit: bool) -> f32 {
    match (wanted, pursuit) {
        (false, _) => 0.0,
        (true, true) => PURSUIT_DENSITY_SCALE,
        (true, false) => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use blackbox_roads::SplitMix;

    use super::*;

    fn rule(name: &str, rate: f32, max: u32, percent: u32) -> Rule {
        Rule { name: name.into(), rate, max_instances: max, percent }
    }

    struct Cars(Vec<(&'static str, u32)>, u32);

    impl Census for Cars {
        fn active(&self, name: &str) -> u32 {
            self.0.iter().find(|c| c.0 == name).map_or(0, |c| c.1)
        }
        fn others(&self) -> u32 {
            self.1
        }
    }

    #[test]
    fn the_tables_interpolate_and_clamp() {
        assert_eq!(spawn_rate(1.0), 8.0);
        assert_eq!(spawn_rate(0.0), 0.0);
        assert!((spawn_rate(0.75) - 2.0).abs() < 1e-5);
        assert_eq!(spawn_rate(5.0), 8.0);
        assert_eq!(offscreen_distance(1.0), 40.0);
        assert_eq!(offscreen_time(0.0), 12.0);
    }

    #[test]
    fn rare_types_wait_longer_than_common_ones() {
        let rules = [rule("sedan", 3.0, 0, 40), rule("garbage", 60.0, 1, 10)];
        let mut timers = TypeTimers::default();
        timers.reset(&rules, &mut SplitMix(1));
        let census = Cars(vec![], 1);
        // At full density the multiplier is 8: 0.5 s of ticks is 4 s of timer.
        timers.advance(0.5, 1.0);
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.extend(timers.next_type(&rules, 10, &census));
        }
        assert!(seen.contains(&0), "the sedan is chosen: {seen:?}");
        assert!(!seen.contains(&1), "the garbage truck is still waiting: {seen:?}");
        timers.advance(10.0, 1.0);
        let mut later = Vec::new();
        for _ in 0..4 {
            later.extend(timers.next_type(&rules, 10, &census));
        }
        assert!(later.contains(&1));
    }

    #[test]
    fn caps_limit_a_type() {
        let rules = [rule("sedan", 3.0, 0, 40), rule("news", 10.0, 1, 0)];
        let mut timers = TypeTimers { timers: vec![100.0, 100.0], cursor: 0 };
        // One news van already runs: it is at its maximum. With only the player, 9 of 10 slots are free and 40 %
        // of them is 3 cars: three sedans are at their share.
        let full = Cars(vec![("news", 1), ("sedan", 3)], 1);
        assert_eq!(timers.next_type(&rules, 10, &full), None);
        let room = Cars(vec![("news", 1), ("sedan", 2)], 1);
        assert_eq!(timers.next_type(&rules, 10, &room), Some(0));
    }

    #[test]
    fn the_cursor_rotates_through_the_types() {
        let rules = [rule("a", 1.0, 0, 0), rule("b", 1.0, 0, 0), rule("c", 1.0, 0, 0)];
        let mut timers = TypeTimers { timers: vec![100.0; 3], cursor: 0 };
        let census = Cars(vec![], 0);
        let order: Vec<_> = (0..6).filter_map(|_| timers.next_type(&rules, 10, &census)).collect();
        assert_eq!(order, vec![0, 1, 2, 0, 1, 2]);
    }

    #[test]
    fn spawning_resets_every_record_of_the_model() {
        let rules = [rule("a", 1.0, 0, 0), rule("b", 1.0, 0, 0), rule("a", 5.0, 0, 0)];
        let mut timers = TypeTimers { timers: vec![9.0; 3], cursor: 0 };
        timers.spawned(&rules, "a");
        assert_eq!(timers.timers, vec![0.0, 9.0, 0.0]);
    }

    #[test]
    fn density_follows_the_game_state() {
        assert_eq!(density(false, false), 0.0);
        assert_eq!(density(true, false), 1.0);
        assert_eq!(density(true, true), PURSUIT_DENSITY_SCALE);
    }
}
