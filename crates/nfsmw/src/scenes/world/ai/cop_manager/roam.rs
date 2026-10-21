//! The free-roam 911 call: the damage and the hard traffic hits the player makes while no cop chases, the call they
//! raise and the timer it sets. Spec: `docs/specs/ai-pursuit-heat.md` section 5.3, `docs/specs/ai-pursuit-speech.md`
//! section 8, `docs/provenance/ai-pursuit-911.md`.
//!
//! The original keeps the counters in its speech system: the cost to state of every smackable the player hits
//! (`CTS911`) and the traffic hits with intensity at least `MinIntensityTrafficSmash` (`TrafficHits911`), both cleared
//! while a pursuit is active or searching. When either passes its heat row's figure in free roam the dispatcher says the
//! 911 report and, when the line has been said, the player's 911 timer is set to `Lifetime911` seconds and the cop lockout
//! is cleared, so a patrol cop that sees the player starts a chase whatever the speed. The counters here follow the same
//! rule and are fed to the speech machine as events; the machine decides and speaks, and the call starts when it answers
//! ([`Roam911::start_call`]). A rewrite fallback raises the call itself when the speech never answers (speech off, no
//! audio device), so the cops are not out of reach of the rule for lack of a sound card.

use std::collections::HashMap;

/// The same object counts again only after this many seconds without contact. **[inferred]** The original counts each
/// collision callback; the rewrite's contact is continuous while the car rests against an object.
const SAME_OBJECT_GAP: f32 = 1.0;
/// Seconds after the thresholds are met, with no answer from the speech, before the world raises the call itself.
/// **[inferred]** (rewrite only).
pub const FALLBACK_AFTER: f32 = 20.0;
/// Metres per second in a mile per hour.
const MPH: f32 = 0.447_04;

/// How hard a hit on traffic is judged (`speechtune`, miles per hour, `docs/specs/ai-pursuit-speech.md` section 7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// `PlayerSmashSpeedRange`: the closing speeds (m/s) that map to intensity 0 and 1.
    pub smash_low: f32,
    pub smash_high: f32,
    /// `MinIntensityTrafficSmash`: the intensity a traffic hit needs to count for the 911.
    pub min_traffic: f32,
}

impl Default for Tuning {
    /// The install's values (10 to 30 mph, 0.5).
    fn default() -> Self {
        Self { smash_low: 10.0 * MPH, smash_high: 30.0 * MPH, min_traffic: 0.5 }
    }
}

impl Tuning {
    /// The tuning of the `speechtune` thresholds.
    pub fn of(tune: &nfsmw_data::speech_chase::ChaseTune) -> Self {
        let [low, high] = tune.player_smash_speed_range;
        Self { smash_low: low * MPH, smash_high: high * MPH, min_traffic: tune.min_intensity_traffic_smash }
    }

    /// The intensity of a hit that closes at `speed` m/s, 0 to 1.
    pub fn intensity(&self, speed: f32) -> f32 {
        let span = (self.smash_high - self.smash_low).max(f32::EPSILON);
        ((speed - self.smash_low) / span).clamp(0.0, 1.0)
    }
}

/// What a heat row says about the call.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Level {
    /// `CTSFor911` (0 or less: never).
    pub cost: i32,
    /// `NumCiviHitsFor911` (0 or less: never).
    pub hits: i32,
    /// `Lifetime911`, seconds.
    pub lifetime: f32,
}

impl Level {
    /// The figures of a heat row's speech fields.
    pub fn of(speech: &nfsmw_data::speech_chase::LevelSpeech) -> Self {
        Self { cost: speech.cts_for_911, hits: speech.hits_for_911, lifetime: speech.lifetime_911 }
    }
}

/// What the player did, as the speech machine is told.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The player hit an object that adds to the damage bill.
    Smashed { cost: u32 },
    /// The player hit a traffic car, closing at this speed (m/s).
    HitTraffic { closing: f32 },
}

/// What was touched: a prop or a traffic car.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Touched {
    Prop(u32),
    Car(u32),
}

/// The 911 counters, timer and fallback.
#[derive(Debug, Default)]
pub struct Roam911 {
    tuning: Tuning,
    levels: Vec<Level>,
    heat: u32,
    clock: f32,
    /// `CTS911` and `TrafficHits911`.
    cost: u32,
    hits: u32,
    /// The player's 911 timer, seconds left; it runs down all the time (`m911CallTimer`).
    timer: f32,
    /// A 911 call was made since the cops were last locked out (the speech's `DISP911_ACTIVE`).
    reported: bool,
    /// Seconds the thresholds have held with no call.
    waiting: f32,
    chasing: bool,
    touched: HashMap<Touched, f32>,
    events: Vec<Event>,
    restarted: bool,
    calls: u32,
}

impl Roam911 {
    /// Gives the thresholds of the install: the hit judgement and the heat rows (index = heat - 1).
    pub fn set_data(&mut self, tuning: Tuning, levels: Vec<Level>) {
        self.tuning = tuning;
        self.levels = levels;
    }

    /// The hit judgement.
    pub fn tuning(&self) -> Tuning {
        self.tuning
    }

    /// The row of the session heat.
    fn level(&self) -> Level {
        let last = self.levels.len().saturating_sub(1);
        self.levels.get((self.heat.max(1) as usize - 1).min(last)).copied().unwrap_or_default()
    }

    /// Whether this contact is a new one: the object was not touched during the last [`SAME_OBJECT_GAP`] seconds. The
    /// contact is remembered either way.
    fn fresh(&mut self, what: Touched) -> bool {
        let last = self.touched.insert(what, self.clock);
        last.is_none_or(|at| self.clock - at >= SAME_OBJECT_GAP)
    }

    /// The player hit prop `id`, which adds `cost` to the damage bill.
    pub fn note_prop(&mut self, id: u32, cost: u32) {
        if self.chasing || cost == 0 || !self.fresh(Touched::Prop(id)) {
            return;
        }
        self.cost = self.cost.saturating_add(cost);
        self.events.push(Event::Smashed { cost });
    }

    /// The player hit the traffic car `car`, closing at `closing` m/s.
    pub fn note_traffic(&mut self, car: u32, closing: f32) {
        if self.chasing || !closing.is_finite() || !self.fresh(Touched::Car(car)) {
            return;
        }
        if self.tuning.intensity(closing) >= self.tuning.min_traffic {
            self.hits += 1;
        }
        self.events.push(Event::HitTraffic { closing });
    }

    /// Whether the counters have reached the heat row's figures.
    pub fn threshold_met(&self) -> bool {
        let level = self.level();
        let over_cost = level.cost > 0 && self.cost >= level.cost as u32;
        let over_hits = level.hits > 0 && self.hits >= level.hits as u32;
        over_cost || over_hits
    }

    /// Advances the clocks by `dt` seconds. `chasing`: a cop chases the player (or the arrest is on): the counters are
    /// cleared. `heat` is the session heat that picks the row. Returns the call's lifetime when the fallback raises the
    /// call.
    pub fn tick(&mut self, dt: f32, chasing: bool, heat: u32) -> Option<f32> {
        if !(dt > 0.0 && dt.is_finite()) {
            return None;
        }
        self.heat = heat;
        self.clock += dt;
        self.timer = (self.timer - dt).max(0.0);
        self.chasing = chasing;
        if chasing {
            self.cost = 0;
            self.hits = 0;
            self.waiting = 0.0;
            self.events.clear();
            return None;
        }
        if self.touched.len() > 64 {
            let clock = self.clock;
            self.touched.retain(|_, at| clock - *at < SAME_OBJECT_GAP);
        }
        if self.reported || !self.threshold_met() {
            self.waiting = 0.0;
            return None;
        }
        self.waiting += dt;
        match self.waiting >= FALLBACK_AFTER {
            true => Some(self.level().lifetime),
            false => None,
        }
    }

    /// The call was made (the dispatcher's report was said, or the fallback ran out): the 911 timer is set to `secs`
    /// (`Set911CallTime` keeps the longer one). Returns false when a call was already made since the last lockout.
    pub fn start_call(&mut self, secs: f32) -> bool {
        if self.reported {
            return false;
        }
        self.reported = true;
        self.waiting = 0.0;
        self.calls += 1;
        self.timer = self.timer.max(secs.max(0.0));
        true
    }

    /// The cops were locked out again (an evade, a paid ticket): the speech restarts, a new call may be made.
    pub fn restart(&mut self) {
        self.cost = 0;
        self.hits = 0;
        self.waiting = 0.0;
        self.reported = false;
        self.restarted = true;
        self.events.clear();
    }

    /// Whether the player's 911 timer runs: a patrol cop that sees the player chases it whatever the speed.
    pub fn active(&self) -> bool {
        self.timer > 0.0
    }

    /// Seconds left on the 911 timer.
    #[cfg(test)]
    pub fn timer(&self) -> f32 {
        self.timer
    }

    /// The events since the last call, for the speech machine.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Whether the speech machine was restarted since the last call (once).
    pub fn take_restarted(&mut self) -> bool {
        std::mem::take(&mut self.restarted)
    }

    /// The counters have something in them, or a call waits for its answer: the speech machine should be stepped.
    pub fn armed(&self) -> bool {
        self.cost > 0 || self.hits > 0 || !self.events.is_empty()
    }

    /// The heat row's figures at the session heat.
    pub fn row(&self) -> Level {
        self.level()
    }

    /// A line for the console.
    pub fn status(&self) -> String {
        let level = self.level();
        let call = match self.active() {
            true => format!("911 call on for {:.0} s more", self.timer),
            false => "no 911 call".to_owned(),
        };
        let waiting = match self.waiting > 0.0 {
            true => format!(", the speech has not answered for {:.0} s", self.waiting),
            false => String::new(),
        };
        format!(
            "{call}; damage {} of {} to call, traffic hits {} of {}; {} calls so far{}{waiting}",
            self.cost,
            level.cost,
            self.hits,
            level.hits,
            self.calls,
            if self.reported { ", reported" } else { "" },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roam() -> Roam911 {
        let mut roam = Roam911::default();
        let rows = vec![Level { cost: 400, hits: 3, lifetime: 45.0 }, Level { cost: 600, hits: 4, lifetime: 45.0 }];
        roam.set_data(Tuning::default(), rows);
        roam.tick(0.1, false, 1);
        roam
    }

    #[test]
    fn smashed_objects_add_up_to_the_rows_cost() {
        let mut roam = roam();
        roam.note_prop(1, 150);
        roam.note_prop(2, 150);
        assert!(!roam.threshold_met(), "300 of 400");
        roam.note_prop(3, 100);
        assert!(roam.threshold_met());
        assert_eq!(roam.take_events().len(), 3, "each hit is told to the speech");
    }

    #[test]
    fn one_object_counts_once_while_the_car_rests_against_it() {
        let mut roam = roam();
        for _ in 0..30 {
            roam.note_prop(7, 150);
            roam.tick(0.05, false, 1);
        }
        assert!(!roam.threshold_met(), "30 contact steps over 1.5 s are one touch and a half");
        assert!(roam.cost <= 300, "{}", roam.cost);
        roam.tick(1.5, false, 1);
        roam.note_prop(7, 150);
        assert!(roam.cost >= 300, "a fresh touch counts again");
    }

    #[test]
    fn a_free_object_costs_nothing() {
        let mut roam = roam();
        roam.note_prop(1, 0);
        assert!(roam.take_events().is_empty());
        assert_eq!(roam.cost, 0);
    }

    #[test]
    fn only_hard_traffic_hits_count_and_the_speech_hears_them_all() {
        let mut roam = roam();
        let soft = 12.0 * MPH;
        let hard = 25.0 * MPH;
        roam.note_traffic(1, soft);
        assert_eq!(roam.hits, 0, "a graze is below the intensity");
        roam.note_traffic(2, hard);
        roam.note_traffic(3, hard);
        assert!(!roam.threshold_met());
        roam.note_traffic(4, hard);
        assert!(roam.threshold_met(), "three hard hits");
        assert_eq!(roam.take_events().len(), 4);
    }

    #[test]
    fn the_same_car_is_one_hit_until_it_is_let_go() {
        let mut roam = roam();
        roam.note_traffic(1, 30.0 * MPH);
        roam.tick(0.5, false, 1);
        roam.note_traffic(1, 30.0 * MPH);
        assert_eq!(roam.hits, 1);
        roam.tick(1.2, false, 1);
        roam.note_traffic(1, 30.0 * MPH);
        assert_eq!(roam.hits, 2);
    }

    #[test]
    fn the_intensity_maps_the_speech_range() {
        let tuning = Tuning::default();
        assert_eq!(tuning.intensity(5.0 * MPH), 0.0);
        assert!((tuning.intensity(20.0 * MPH) - 0.5).abs() < 1e-5);
        assert_eq!(tuning.intensity(80.0 * MPH), 1.0);
    }

    #[test]
    fn a_chase_clears_the_counters_and_blocks_new_ones() {
        let mut roam = roam();
        roam.note_prop(1, 300);
        roam.tick(0.1, true, 1);
        assert_eq!(roam.cost, 0);
        roam.note_prop(2, 300);
        assert_eq!(roam.cost, 0, "while a cop chases the player, nothing adds up");
        roam.tick(0.1, false, 1);
        roam.note_prop(2, 300);
        assert_eq!(roam.cost, 300);
    }

    #[test]
    fn the_heat_picks_the_row() {
        let mut roam = roam();
        roam.note_prop(1, 500);
        assert!(roam.threshold_met(), "400 at heat 1");
        roam.tick(0.1, false, 2);
        assert!(!roam.threshold_met(), "600 at heat 2");
        roam.tick(0.1, false, 40);
        assert!(!roam.threshold_met(), "a heat beyond the table reads the last row");
    }

    #[test]
    fn the_call_sets_the_timer_which_runs_down_and_is_made_once() {
        let mut roam = roam();
        assert!(!roam.active());
        assert!(roam.start_call(45.0));
        assert!(roam.active());
        assert!(!roam.start_call(45.0), "one call until the cops are locked out again");
        roam.tick(44.0, false, 1);
        assert!(roam.active());
        roam.tick(1.5, false, 1);
        assert!(!roam.active(), "after Lifetime911 s");
    }

    #[test]
    fn a_longer_timer_is_kept() {
        let mut roam = roam();
        roam.start_call(180.0);
        roam.restart();
        roam.start_call(45.0);
        assert!((roam.timer() - 180.0).abs() < 0.2, "{}", roam.timer());
    }

    #[test]
    fn a_lockout_restarts_the_speech_and_allows_another_call() {
        let mut roam = roam();
        roam.note_prop(1, 500);
        roam.start_call(45.0);
        roam.restart();
        assert!(roam.take_restarted());
        assert!(!roam.take_restarted(), "once");
        assert!(!roam.threshold_met(), "the counters start again");
        assert!(roam.start_call(45.0));
    }

    #[test]
    fn the_world_raises_the_call_when_the_speech_never_answers() {
        let mut roam = roam();
        roam.note_prop(1, 500);
        assert_eq!(roam.tick(FALLBACK_AFTER - 1.0, false, 1), None);
        assert_eq!(roam.tick(1.5, false, 1), Some(45.0));
        assert!(roam.start_call(45.0));
        assert_eq!(roam.tick(5.0, false, 1), None, "a call that was made is not made again");
    }

    #[test]
    fn a_call_that_the_speech_made_stops_the_fallback() {
        let mut roam = roam();
        roam.note_prop(1, 500);
        roam.tick(5.0, false, 1);
        roam.start_call(45.0);
        assert_eq!(roam.tick(FALLBACK_AFTER, false, 1), None);
    }

    #[test]
    fn the_status_names_the_call_and_the_counters() {
        let mut roam = roam();
        roam.note_prop(1, 100);
        assert!(roam.status().contains("damage 100 of 400"), "{}", roam.status());
        roam.start_call(45.0);
        assert!(roam.status().contains("911 call on"), "{}", roam.status());
    }

    #[test]
    fn a_bad_step_changes_nothing() {
        let mut roam = roam();
        roam.start_call(45.0);
        assert_eq!(roam.tick(f32::NAN, false, 1), None);
        assert_eq!(roam.tick(-1.0, false, 1), None);
        assert!(roam.timer() > 44.0);
    }
}
