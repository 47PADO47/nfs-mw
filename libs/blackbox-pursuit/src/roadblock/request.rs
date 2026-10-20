//! When a roadblock is wanted: the two-step lottery of `RequestRoadBlock` and the latch the cop manager keeps on its
//! result. Spec: `docs/specs/ai-pursuit-roadblocks.md` section 1 (checked against the PC binary, see the provenance).
//!
//! The pursuit rolls at most once per 8 to 12 s. A successful roll only arms the next request; the request after it
//! (8 to 12 s later) is the one that reaches the manager. The manager latches it and tries to build a roadblock on every
//! run until one stands, the pursuit stops, or a roadblock exists some other way. While the latch holds, the manager
//! spawns no new pursuit cops. Pure: the caller supplies the random numbers and the clock.

/// The roadblock columns of one `pursuitlevels` row, in percent and metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Odds {
    /// `roadblockprobability`: the roll's chance while the target is in sight.
    pub probability: f32,
    /// `roadblockspikechance`: the chance that a roadblock takes the spike layouts.
    pub spike_chance: f32,
    /// `SearchModeRoadblockChance`: the chance at the last known position while the target is not in sight.
    pub search_chance: f32,
    /// `SearchModeRoadblockRadius`: the chance falls to zero this far from the last known position.
    pub search_radius: f32,
}

/// The install's rows, heat 1 to 10 (`pursuitlevels`, **[verified]**: the installed test
/// `installed_level_rows_match_the_roadblock_and_helicopter_tables` compares every cell). Columns: probability, spike
/// chance, search chance, search radius in metres.
const ROWS: [(f32, f32, f32, f32); 10] = [
    (0.0, 0.0, 0.0, 1000.0),
    (10.0, 0.0, 3.0, 1000.0),
    (25.0, 0.0, 5.0, 2000.0),
    (30.0, 70.0, 7.0, 5000.0),
    (40.0, 80.0, 10.0, 10000.0),
    (50.0, 85.0, 12.0, 10000.0),
    (35.0, 85.0, 12.0, 10000.0),
    (50.0, 0.0, 60.0, 3500.0),
    (100.0, 100.0, 0.0, 4000.0),
    (80.0, 0.0, 80.0, 5000.0),
];

/// The odds of heat `level` (1 to 10; anything else has none).
pub fn odds(level: u32) -> Odds {
    let row = level.checked_sub(1).and_then(|i| ROWS.get(i as usize)).copied().unwrap_or((0.0, 0.0, 0.0, 1000.0));
    Odds { probability: row.0, spike_chance: row.1, search_chance: row.2, search_radius: row.3 }
}

/// The chance (percent) that a roll succeeds: the row's probability in sight, otherwise the search chance scaled by how
/// near the target still is to the last known position. It goes negative beyond the radius, which never succeeds.
pub fn chance(odds: &Odds, in_sight: bool, last_known_distance: f32) -> f32 {
    if in_sight {
        return odds.probability;
    }
    odds.search_chance * (odds.search_radius - last_known_distance) / odds.search_radius
}

/// What the pursuit knows when it is asked.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ask {
    pub busted: bool,
    pub bailed: bool,
    /// The pursuit has a roadblock attached (one with cars left; see [`super::standing`]).
    pub roadblock_exists: bool,
    /// Seconds of the pursuit so far.
    pub pursuit_time: f32,
    /// `MinimumSupportDelay` of the pursuit's support row.
    pub min_delay: f32,
    /// Some cop (or roadblock car) has seen the target within the sight grace.
    pub in_sight: bool,
    /// Metres from the last known position to the target.
    pub last_known_distance: f32,
}

/// The pursuit's roadblock timer and the armed flag (`mRoadBlockTimer`, `mNextRoadblockRequest`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Requester {
    timer: f32,
    armed: bool,
}

impl Default for Requester {
    fn default() -> Self {
        Self { timer: 0.0, armed: false }
    }
}

impl Requester {
    /// Seconds the timer waits after a roll: `RETRY_BASE` plus a random part of `RETRY_SPREAD`.
    pub const RETRY_BASE: f32 = 8.0;
    pub const RETRY_SPREAD: f32 = 4.0;
    /// The wait that a heavy support request imposes (`mRoadBlockTimer = 15`).
    pub const HEAVY_WAIT: f32 = 15.0;

    /// The pursuit task counts the timer down.
    pub fn tick(&mut self, dt: f32) {
        self.timer -= dt;
    }

    /// A heavy support request was chosen: the lottery waits.
    pub fn heavy_chosen(&mut self) {
        self.timer = Self::HEAVY_WAIT;
    }

    /// A roll succeeded and the next request will go through (`PendingRoadBlockRequest`; the speech reads it).
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// `RequestRoadBlock`: returns whether this request reaches the manager. `rolls` are two uniform numbers in
    /// `0..=1`: the first sets the next wait, the second is the percentage roll.
    pub fn request(&mut self, ask: &Ask, odds: &Odds, rolls: [f32; 2]) -> bool {
        if ask.busted || ask.bailed || ask.roadblock_exists || self.timer >= 0.0 || ask.min_delay > ask.pursuit_time {
            return false;
        }
        self.timer = Self::RETRY_BASE + rolls[0].clamp(0.0, 1.0) * Self::RETRY_SPREAD;
        let reaches = self.armed;
        let percent = rolls[1].clamp(0.0, 1.0) * 100.0;
        self.armed = percent < chance(odds, ask.in_sight, ask.last_known_distance);
        reaches
    }
}

/// The requester and the manager's latch on its result (`mIPursuitWithLatchedRoadblockReq`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Dispatcher {
    requester: Requester,
    latched: bool,
}

impl Dispatcher {
    pub fn tick(&mut self, dt: f32) {
        self.requester.tick(dt);
    }

    pub fn heavy_chosen(&mut self) {
        self.requester.heavy_chosen();
    }

    pub fn is_armed(&self) -> bool {
        self.requester.is_armed()
    }

    /// A request is latched: the manager spawns no new pursuit cops until it is met (`IsCopRequestPending`).
    pub fn is_latched(&self) -> bool {
        self.latched
    }

    /// One manager run: asks the pursuit unless a request is latched, and says whether a roadblock should be built now.
    /// A latch that finds a roadblock standing lets go (the manager clears it when the pursuit has a roadblock).
    pub fn poll(&mut self, ask: &Ask, odds: &Odds, rolls: [f32; 2]) -> bool {
        if !self.latched && self.requester.request(ask, odds, rolls) {
            self.latched = true;
        }
        if self.latched && ask.roadblock_exists {
            self.latched = false;
        }
        self.latched
    }

    /// The roadblock was built: the latch is met.
    pub fn built(&mut self) {
        self.latched = false;
    }

    /// The chase ended or a new one began.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask() -> Ask {
        Ask {
            busted: false,
            bailed: false,
            roadblock_exists: false,
            pursuit_time: 100.0,
            min_delay: 35.0,
            in_sight: true,
            last_known_distance: 0.0,
        }
    }

    fn heat_five() -> Odds {
        odds(5)
    }

    #[test]
    fn the_rows_are_the_installs() {
        assert_eq!(odds(1).probability, 0.0);
        assert_eq!(odds(4), Odds { probability: 30.0, spike_chance: 70.0, search_chance: 7.0, search_radius: 5000.0 });
        assert_eq!(odds(9).probability, 100.0);
        assert_eq!(odds(10).search_chance, 80.0);
        assert_eq!(odds(0).probability, 0.0, "no heat has no odds");
        assert_eq!(odds(11).probability, 0.0);
    }

    #[test]
    fn the_spike_table_agrees_with_the_layout_picker() {
        for level in 1..=10 {
            assert_eq!(odds(level).spike_chance, super::super::spike_chance(level), "heat {level}");
        }
    }

    #[test]
    fn a_roll_arms_the_next_request_and_only_that_one_reaches_the_manager() {
        let mut r = Requester::default();
        r.tick(0.25);
        assert!(!r.request(&ask(), &heat_five(), [0.0, 0.0]), "the first request only rolls");
        assert!(r.is_armed(), "0 percent is under the 40 of heat 5");
        r.tick(7.9);
        assert!(!r.request(&ask(), &heat_five(), [0.0, 0.0]), "the timer is still 8 s");
        r.tick(0.2);
        assert!(r.request(&ask(), &heat_five(), [0.0, 0.99]), "the armed request goes through");
        assert!(!r.is_armed(), "and the new roll missed (99 is over 40)");
    }

    #[test]
    fn a_missed_roll_leaves_nothing_armed() {
        let mut r = Requester::default();
        r.tick(1.0);
        r.request(&ask(), &heat_five(), [0.5, 0.99]);
        assert!(!r.is_armed());
    }

    #[test]
    fn the_wait_after_a_roll_is_eight_to_twelve_seconds() {
        let mut r = Requester::default();
        r.tick(1.0);
        r.request(&ask(), &heat_five(), [1.0, 0.99]);
        r.tick(11.9);
        assert!(!r.request(&ask(), &heat_five(), [0.0, 0.0]), "12 s have not passed");
        r.tick(0.2);
        r.request(&ask(), &heat_five(), [0.0, 0.0]);
        assert!(r.is_armed(), "12 s have passed: the roll was made");
    }

    #[test]
    fn the_gates_refuse_before_the_timer_is_spent() {
        let heat = heat_five();
        for refuse in [
            Ask { busted: true, ..ask() },
            Ask { bailed: true, ..ask() },
            Ask { roadblock_exists: true, ..ask() },
            Ask { pursuit_time: 10.0, ..ask() },
        ] {
            let mut r = Requester::default();
            r.tick(1.0);
            assert!(!r.request(&refuse, &heat, [0.0, 0.0]));
            assert!(!r.is_armed(), "a refused request does not roll");
            let _ = r.request(&ask(), &heat, [0.0, 0.0]);
            assert!(r.is_armed(), "and does not spend the timer");
        }
    }

    #[test]
    fn out_of_sight_the_chance_falls_with_the_distance_from_the_last_known_place() {
        let heat = odds(8);
        assert_eq!(heat.search_radius, 3500.0);
        assert_eq!(chance(&heat, false, 0.0), 60.0);
        assert_eq!(chance(&heat, false, 1750.0), 30.0);
        assert_eq!(chance(&heat, false, 3500.0), 0.0);
        assert!(chance(&heat, false, 4000.0) < 0.0, "beyond the radius it never succeeds");
        assert_eq!(chance(&heat, true, 900.0), 50.0, "in sight the row's probability holds");
    }

    #[test]
    fn a_heavy_request_holds_the_lottery_for_fifteen_seconds() {
        let mut r = Requester::default();
        r.heavy_chosen();
        r.tick(14.9);
        assert!(!r.request(&ask(), &heat_five(), [0.0, 0.0]));
        assert!(!r.is_armed());
        r.tick(0.2);
        r.request(&ask(), &heat_five(), [0.0, 0.0]);
        assert!(r.is_armed());
    }

    #[test]
    fn the_latch_holds_until_a_roadblock_is_built() {
        let mut d = Dispatcher::default();
        d.tick(1.0);
        assert!(!d.poll(&ask(), &heat_five(), [0.0, 0.0]), "the roll arms, nothing is latched");
        d.tick(12.1);
        assert!(d.poll(&ask(), &heat_five(), [0.0, 0.99]), "the armed request latches");
        assert!(d.is_latched());
        assert!(d.poll(&ask(), &heat_five(), [0.0, 0.0]), "it is kept for the next run");
        d.built();
        assert!(!d.is_latched());
    }

    #[test]
    fn a_latch_lets_go_when_a_roadblock_already_stands() {
        let mut d = Dispatcher::default();
        d.tick(1.0);
        d.poll(&ask(), &heat_five(), [0.0, 0.0]);
        d.tick(12.1);
        assert!(d.poll(&ask(), &heat_five(), [0.0, 0.99]));
        let standing = Ask { roadblock_exists: true, ..ask() };
        assert!(!d.poll(&standing, &heat_five(), [0.0, 0.0]));
        assert!(!d.is_latched());
    }

    #[test]
    fn heat_one_never_arms() {
        let mut d = Dispatcher::default();
        let heat = odds(1);
        for _ in 0..100 {
            d.tick(13.0);
            assert!(!d.poll(&ask(), &heat, [0.0, 0.0]));
        }
        assert!(!d.is_armed(), "0 is not under 0");
    }
}
