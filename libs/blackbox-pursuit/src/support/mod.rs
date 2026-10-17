//! Ground support requests: the roll that picks the support a chase asks for (spec `ai-pursuit-formations.md` section 6),
//! the request's state ([`order`]), the cars it takes ([`cars`]), where they stand ([`placement`]), the shared ram delay
//! ([`delay`]) and the head-on ram's drive ([`ram`]). Spec `ai-pursuit-support.md`.
//! The per-level options are the install's data and come from the caller; this module applies the order of the rolls.
//! Spawning the cars and driving them through the world is in the game (`crates/nfsmw/.../ai/support`).

pub mod cars;
pub mod delay;
pub mod order;
pub mod placement;
pub mod ram;

pub use cars::Car;
pub use delay::RamDelay;
pub use order::{Order, State};
pub use placement::Search;

/// The support strategies of the spec's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    EBrake,
    CoordinatedEBrake,
    Ram,
    HeavyRoadblock,
    CrossFollow,
    CrossBrake,
    CrossPlusVBlock,
}

impl Strategy {
    /// A heavy support (the SUVs): the roll that comes before the leader's.
    pub fn is_heavy(self) -> bool {
        matches!(self, Strategy::EBrake | Strategy::CoordinatedEBrake | Strategy::Ram | Strategy::HeavyRoadblock)
    }
}

/// One option of a level: its strategy, its chance (percent, 0 to 100) and how long the support lasts (seconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Option_ {
    pub strategy: Strategy,
    pub chance: f32,
    pub duration: f32,
}

/// The request the roll picked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Request {
    pub strategy: Strategy,
    pub duration: f32,
}

/// The seconds a heavy roadblock waits before it is placed (`roadblock_timer`, spec section 6).
pub const ROADBLOCK_DELAY: f32 = 15.0;

/// The base roll (`RequestGroundSupport`, after the priority check): a random number in `0..100` is spent down the heavy
/// options (only when no roadblock stands), then down the leader options. `rand` is uniform in `0..=1`.
pub fn roll(heavy: &[Option_], leaders: &[Option_], rand: f32, roadblock_exists: bool) -> Option<Request> {
    let mut rand = rand.clamp(0.0, 1.0) * 100.0;
    if !roadblock_exists {
        for option in heavy {
            rand -= option.chance;
            if rand < 0.0 {
                return Some(Request { strategy: option.strategy, duration: option.duration });
            }
        }
    }
    for option in leaders {
        rand -= option.chance;
        if rand < 0.0 {
            return Some(Request { strategy: option.strategy, duration: option.duration });
        }
    }
    None
}

/// The priority check: the first leader option whose priority time has passed and whose priority chance beats `rand`
/// (uniform in `0..=1`). Done once per heat level, the first time the pursuit has lasted that long.
pub fn priority(leaders: &[(Option_, f32, f32)], pursuit_time: f32, rand: f32) -> Option<Request> {
    let rand = rand.clamp(0.0, 1.0) * 100.0;
    leaders.iter().find_map(|&(option, priority_chance, priority_time)| {
        (priority_time < pursuit_time && priority_chance > rand)
            .then_some(Request { strategy: option.strategy, duration: option.duration })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAM: Option_ = Option_ { strategy: Strategy::Ram, chance: 25.0, duration: 30.0 };
    const ROADBLOCK: Option_ = Option_ { strategy: Strategy::HeavyRoadblock, chance: 25.0, duration: 40.0 };
    const CROSS: Option_ = Option_ { strategy: Strategy::CrossFollow, chance: 5.0, duration: 2000.0 };

    #[test]
    fn a_low_roll_takes_the_first_heavy_option() {
        let r = roll(&[RAM, ROADBLOCK], &[CROSS], 0.1, false).expect("picked");
        assert_eq!(r.strategy, Strategy::Ram);
        assert_eq!(r.duration, 30.0);
    }

    #[test]
    fn a_roll_past_the_heavy_chances_falls_to_the_leaders() {
        // The heavy options take 50 of the 100 points; a roll of 52 leaves 2, which is under the leader's 5.
        let r = roll(&[RAM, ROADBLOCK], &[CROSS], 0.52, false).expect("picked");
        assert_eq!(r.strategy, Strategy::CrossFollow);
    }

    #[test]
    fn a_standing_roadblock_skips_the_heavy_options() {
        let r = roll(&[RAM, ROADBLOCK], &[], 0.1, true);
        assert!(r.is_none(), "no leader options and the heavy ones are skipped");
    }

    #[test]
    fn no_request_when_the_rolls_miss_everything() {
        assert!(roll(&[RAM], &[CROSS], 0.9, false).is_none());
    }

    #[test]
    fn the_priority_check_needs_the_pursuit_to_have_lasted_its_time() {
        let leaders = [(CROSS, 100.0, 120.0)];
        assert!(priority(&leaders, 100.0, 0.5).is_none(), "not yet 120 s");
        assert!(priority(&leaders, 130.0, 0.5).is_some(), "after 120 s with 100 percent");
    }

    #[test]
    fn the_priority_chance_must_beat_the_roll() {
        let leaders = [(CROSS, 10.0, 0.0)];
        assert!(priority(&leaders, 50.0, 0.5).is_none(), "50 is not under 10");
        assert!(priority(&leaders, 50.0, 0.05).is_some(), "5 is under 10");
    }

    #[test]
    fn only_the_suv_strategies_are_heavy() {
        assert!(Strategy::Ram.is_heavy());
        assert!(Strategy::HeavyRoadblock.is_heavy());
        assert!(!Strategy::CrossFollow.is_heavy());
        assert!(!Strategy::CrossPlusVBlock.is_heavy());
    }
}
