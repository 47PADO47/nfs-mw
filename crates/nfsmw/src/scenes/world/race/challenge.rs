//! The pursuit challenges (`challenge` races, `docs/specs/career-scripts-events.md` section 6.1): the player starts on
//! the road at the race's rolling start speed with a chase at the race's heat, and has to reach the race's statistic
//! (`MilestoneName` at `ThreshholdValue`) and then get away. The challenge is won by escaping once the value is
//! reached; being busted, or escaping short of it, loses it. The rules are the spec's; the statistics come from the
//! chase the world already runs.

use nfsmw_data::gameplay::Race;

use super::{Banner, RaceResult, RaceSession, Stage};
use crate::scenes::world::{WorldScene, space};
use crate::viewer::PursuitReport;

/// A roadblock counts as dodged once the car has been this close to it, metres, and then gone this far past it.
const NEAR: f32 = 40.0;
const PAST: f32 = 90.0;
/// Seconds the "challenge complete" line stays.
const BANNER_SECS: f32 = 3.0;

/// The statistic a challenge asks for (`MilestoneName`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) enum Stat {
    PursuitLength,
    CopsDamaged,
    CostToState,
    Bounty,
    Infractions,
    RoadblocksDodged,
    /// The seconds the escape took: the challenge is met when the chase ended in an escape within the threshold.
    EvasionTime,
    /// Spike strips driven past with no tire punctured.
    SpikesDodged,
}

impl Stat {
    /// The statistic of a `MilestoneName`.
    pub(super) fn of(name: &str) -> Option<Self> {
        Some(match name {
            "pursuit_length" => Stat::PursuitLength,
            "cops_damaged" => Stat::CopsDamaged,
            "cost_to_state_in_pursuit" => Stat::CostToState,
            "bounty_in_pursuit" => Stat::Bounty,
            "total_infractions" => Stat::Infractions,
            "roadblocks_dodged" => Stat::RoadblocksDodged,
            "pursuit_evasion_time" => Stat::EvasionTime,
            "tire_spikes_dodged" => Stat::SpikesDodged,
            _ => return None,
        })
    }
}

/// One challenge being run.
#[derive(Debug, Clone)]
pub(in crate::scenes::world) struct Challenge {
    pub stat: Stat,
    pub threshold: f32,
    /// The chase's heat level.
    pub heat: u32,
    /// The chase was started.
    pub began: bool,
    /// The statistic reached its threshold (the HUD says so once).
    pub reached: bool,
    /// Roadblock sites the car has come near and not yet passed, by site index; and the dodges counted.
    roads: Dodges,
    spikes: Dodges,
    pub roadblocks_dodged: u32,
    pub spikes_dodged: u32,
    /// The chase's numbers as of the last frame, for the result screen.
    pub last: Option<PursuitReport>,
}

impl Challenge {
    pub(super) fn new(stat: Stat, threshold: f32, heat: u32) -> Self {
        Self {
            stat,
            threshold,
            heat,
            began: false,
            reached: false,
            roads: Dodges::default(),
            spikes: Dodges::default(),
            roadblocks_dodged: 0,
            spikes_dodged: 0,
            last: None,
        }
    }

    /// Whether `value` meets the threshold. The evasion time is met at or under it, every other one at or over it.
    pub(super) fn met(&self, value: f32) -> bool {
        match self.stat {
            Stat::EvasionTime => value <= self.threshold,
            _ => value >= self.threshold,
        }
    }

    /// The row of the result screen the challenge's statistic is on, and the value it had to reach.
    pub(super) fn goal(&self) -> Option<(usize, f32)> {
        let row = match self.stat {
            Stat::PursuitLength => 0,
            Stat::EvasionTime => 1,
            Stat::CopsDamaged => 2,
            Stat::RoadblocksDodged => 4,
            Stat::CostToState => 5,
            Stat::Bounty | Stat::Infractions | Stat::SpikesDodged => return None,
        };
        Some((row, self.threshold))
    }

    /// Judges one frame: `value` is the statistic now and `reason` how the chase stands. Notes the value reached, and
    /// answers `Some(won)` when the chase is over: an escape with the value reached wins, a bust or an escape short of
    /// it loses. The evasion time is judged at the escape, the other statistics as the chase goes on.
    pub(super) fn judge(&mut self, value: f32, reason: blackbox_pursuit::EndReason) -> Option<bool> {
        use blackbox_pursuit::EndReason;
        let evaded = reason == EndReason::Evaded;
        let now = match self.stat {
            Stat::EvasionTime => evaded && self.met(value),
            _ => self.met(value),
        };
        self.reached |= now;
        match reason {
            EndReason::None => None,
            EndReason::Busted => Some(false),
            EndReason::Evaded => Some(self.reached),
        }
    }

    /// Counts the roadblocks the car has driven past: near a site first, then well beyond it.
    pub(super) fn track_roadblocks(&mut self, sites: &[[f32; 2]], player: [f32; 2]) {
        self.roadblocks_dodged += self.roads.track(sites, player, 0);
    }

    /// Counts the spike strips the car has driven past without a tire of it being punctured on the way: `hits` is the
    /// number of punctures so far **[unconfirmed]** (the original's rule for a strip dodged is not read).
    pub(super) fn track_spikes(&mut self, sites: &[[f32; 2]], player: [f32; 2], hits: u32) {
        self.spikes_dodged += self.spikes.track(sites, player, hits);
    }
}

/// Which sites of a kind the car has come near and passed.
#[derive(Debug, Clone, Default)]
struct Dodges {
    /// The sites the car is near, with the number of punctures when it came near.
    near: Vec<(usize, u32)>,
    passed: Vec<usize>,
}

impl Dodges {
    /// Moves the car among `sites`; returns how many sites it has just passed clean (no puncture since it came near).
    fn track(&mut self, sites: &[[f32; 2]], player: [f32; 2], hits: u32) -> u32 {
        let mut dodged = 0;
        for (i, site) in sites.iter().enumerate() {
            if self.passed.contains(&i) {
                continue;
            }
            let distance = ((site[0] - player[0]).powi(2) + (site[1] - player[1]).powi(2)).sqrt();
            if distance < NEAR && !self.near.iter().any(|(n, _)| *n == i) {
                self.near.push((i, hits));
            }
            let Some((_, at_near)) = self.near.iter().find(|(n, _)| *n == i).copied() else { continue };
            if distance > PAST {
                self.passed.push(i);
                dodged += u32::from(hits == at_near);
            }
        }
        dodged
    }
}

impl WorldScene {
    /// Starts a pursuit challenge: the car goes to the race's start at its rolling speed and the chase begins on the
    /// first frame the car stands (`challenge_running`).
    pub(super) fn start_challenge(&mut self, race: &Race, options: super::RaceOptions) -> Result<String, String> {
        if race.name.ends_with("/epic_pursuit") {
            return Err("the final pursuit ends at its win trigger, which this version does not run".into());
        }
        let params = &race.params;
        let name = params.milestone.as_deref().ok_or("the challenge has no statistic to reach")?;
        let stat = Stat::of(name).ok_or_else(|| format!("the challenge statistic {name:?} is not known"))?;
        let start = race.start.ok_or("the race has no start")?;
        if self.traffic.is_none() {
            return Err("this track has no road network, so no cops can chase".into());
        }
        let heat = params.force_heat_level.clamp(1, 10) as u32;
        let spawn = crate::scenes::world::road::Spawn {
            position: space::to_render(start.position.to_array()),
            heading: super::setup::spawn_heading(start.heading_deg),
        };
        let speed = params.initial_speed.max(0.0);
        if let Some(drive) = self.drive.as_mut() {
            drive.place_exactly(spawn, speed);
        }
        self.view = crate::scenes::world::View::Chase;
        let previous_traffic = self.traffic_manual;
        self.race = Some(RaceSession {
            event: crate::career::race_event_id(race),
            map: crate::hud::MapSource::of_race(
                &crate::career::race_event_id(race),
                params.pursuit_race,
                race.region.as_deref(),
            ),
            name: race.name.clone(),
            kind: nfsmw_data::gameplay::RaceType::Challenge,
            options,
            stage: Stage::Running,
            engine: None,
            plan: Vec::new(),
            setup: None,
            previous_traffic,
            glue: Vec::new(),
            catch_up: false,
            glue_clock: 0.0,
            demo_clock: 0.0,
            lane: Default::default(),
            emu: Vec::new(),
            log_clock: 0.0,
            banner: Banner::default(),
            result: None,
            report: None,
            chain: None,
            cash_value: params.cash_value,
            pool: Vec::new(),
            challenge: Some(Challenge::new(stat, params.threshold, heat)),
            walls: self.raise_race_walls(&race.barriers),
            origin: super::Origin { race: race.clone(), course: None },
        });
        Ok(format!("{}: pursuit challenge, {name} {} at heat {heat}", race.name, params.threshold))
    }

    /// One frame of a challenge: the chase starts once the car stands, the statistic is watched and the end of the chase
    /// decides the result.
    pub(super) fn challenge_running(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        session: &mut RaceSession,
    ) -> bool {
        let Some(mut challenge) = session.challenge.take() else { return false };
        let keep = self.challenge_step(renderer, session, &mut challenge);
        session.challenge = Some(challenge);
        keep
    }

    fn challenge_step(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        session: &mut RaceSession,
        challenge: &mut Challenge,
    ) -> bool {
        if !challenge.began {
            let standing = self.drive.as_ref().is_some_and(|d| d.has_sim() && d.waiting_for_road().is_none());
            if !standing {
                return true;
            }
            challenge.began = true;
            let heat = challenge.heat.to_string();
            match crate::scenes::world::ai::pursuit_command(self, renderer, &[heat.as_str()]) {
                Ok(line) => log::info!("race: {line}"),
                Err(why) => {
                    log::warn!("race {}: the chase did not start: {why}", session.name);
                    return false;
                }
            }
            return true;
        }
        let Some(traffic) = self.traffic.as_ref() else { return false };
        let player = self.drive.as_ref().map(|d| d.position());
        let (secs, reason) = (traffic.pursuit_time(), traffic.end_reason());
        if let Some(position) = player {
            let at = [position[0], position[1]];
            challenge.track_roadblocks(traffic.roadblock_sites(), at);
            challenge.track_spikes(&traffic.spike_sites(), at, traffic.spike_hits());
        }
        let value = match challenge.stat {
            Stat::PursuitLength | Stat::EvasionTime => secs,
            Stat::CopsDamaged => traffic.cops_damaged() as f32,
            Stat::CostToState => traffic.cost_to_state() as f32,
            Stat::Bounty => traffic.rep_normal_points().saturating_add(traffic.rep_destroyed_points()) as f32,
            Stat::Infractions => traffic.infractions().count() as f32,
            Stat::RoadblocksDodged => challenge.roadblocks_dodged as f32,
            Stat::SpikesDodged => challenge.spikes_dodged as f32,
        };
        challenge.last = Some(PursuitReport {
            secs,
            evasion: (reason == blackbox_pursuit::EndReason::Evaded).then_some(secs),
            cops_damaged: traffic.cops_damaged(),
            cops_destroyed: traffic.cops_destroyed(),
            roadblocks_dodged: challenge.roadblocks_dodged,
            cost_to_state: traffic.cost_to_state(),
            infractions: traffic.infractions().count(),
            goal: challenge.goal(),
        });
        let was_reached = challenge.reached;
        let verdict = challenge.judge(value, reason);
        if challenge.reached && !was_reached {
            session.banner = Banner {
                text: "Challenge complete: now get away".into(),
                left: BANNER_SECS,
                serial: session.banner.serial.wrapping_add(1),
            };
            log::info!("race: {}: the challenge value is reached ({value})", session.name);
        }
        let Some(won) = verdict else { return true };
        self.finish_challenge(session, won, secs);
        true
    }

    /// The challenge is over: the career hears of a win, the chase is let go and the result shows.
    fn finish_challenge(&mut self, session: &mut RaceSession, won: bool, secs: f32) {
        let place: u32 = if won { 1 } else { 2 };
        session.result = Some(RaceResult { place, racers: 2, time: won.then_some(secs) });
        session.stage = Stage::Results { left: 3.0 };
        self.note_race_outcome(session);
        session.banner = Banner::default();
        log::info!("race: challenge {} {} after {secs:.1} s", session.name, if won { "won" } else { "lost" });
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.stop_pursuit();
        }
        // Only an escape with the value reached is notified; a lost challenge changes nothing in the career.
        if session.options.career && won {
            self.career_race_finished(&session.event, 1, session.cash_value);
            return;
        }
        if won {
            crate::frontend::challenge_series::record_win(&session.event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_statistics_of_the_install_are_known() {
        for name in [
            "bounty_in_pursuit",
            "cops_damaged",
            "cost_to_state_in_pursuit",
            "pursuit_evasion_time",
            "pursuit_length",
            "roadblocks_dodged",
            "tire_spikes_dodged",
            "total_infractions",
        ] {
            assert!(Stat::of(name).is_some(), "{name}");
        }
        assert_eq!(Stat::of("nonsense"), None);
    }

    #[test]
    fn every_statistic_is_met_from_above_but_the_evasion_time() {
        assert!(Challenge::new(Stat::PursuitLength, 60.0, 3).met(60.0));
        assert!(!Challenge::new(Stat::PursuitLength, 60.0, 3).met(59.0));
        assert!(Challenge::new(Stat::EvasionTime, 300.0, 3).met(250.0));
        assert!(!Challenge::new(Stat::EvasionTime, 300.0, 3).met(301.0));
    }

    #[test]
    fn an_escape_wins_only_with_the_value_reached() {
        use blackbox_pursuit::EndReason;
        let mut c = Challenge::new(Stat::PursuitLength, 60.0, 3);
        assert_eq!(c.judge(30.0, EndReason::None), None);
        assert!(!c.reached);
        assert_eq!(c.judge(61.0, EndReason::None), None);
        assert!(c.reached, "the value is noted while the chase goes on");
        assert_eq!(c.judge(70.0, EndReason::Evaded), Some(true));
        let mut short = Challenge::new(Stat::PursuitLength, 60.0, 3);
        assert_eq!(short.judge(40.0, EndReason::Evaded), Some(false), "an escape short of the value loses");
        let mut caught = Challenge::new(Stat::PursuitLength, 60.0, 3);
        caught.judge(100.0, EndReason::None);
        assert_eq!(caught.judge(100.0, EndReason::Busted), Some(false), "a bust loses even past the value");
    }

    #[test]
    fn the_evasion_time_is_judged_at_the_escape() {
        use blackbox_pursuit::EndReason;
        let mut c = Challenge::new(Stat::EvasionTime, 300.0, 3);
        assert_eq!(c.judge(100.0, EndReason::None), None);
        assert!(!c.reached, "not before the escape");
        assert_eq!(c.judge(250.0, EndReason::Evaded), Some(true));
        let mut slow = Challenge::new(Stat::EvasionTime, 300.0, 3);
        assert_eq!(slow.judge(400.0, EndReason::Evaded), Some(false));
    }

    #[test]
    fn a_roadblock_is_dodged_once_the_car_has_passed_close_by_and_gone_on() {
        let mut c = Challenge::new(Stat::RoadblocksDodged, 2.0, 3);
        let sites = [[0.0, 0.0]];
        c.track_roadblocks(&sites, [200.0, 0.0]);
        assert_eq!(c.roadblocks_dodged, 0, "far away: not yet");
        c.track_roadblocks(&sites, [20.0, 0.0]);
        c.track_roadblocks(&sites, [60.0, 0.0]);
        assert_eq!(c.roadblocks_dodged, 0, "not far enough past");
        c.track_roadblocks(&sites, [-120.0, 0.0]);
        assert_eq!(c.roadblocks_dodged, 1);
        c.track_roadblocks(&sites, [20.0, 0.0]);
        c.track_roadblocks(&sites, [-120.0, 0.0]);
        assert_eq!(c.roadblocks_dodged, 1, "counted once");
    }

    #[test]
    fn a_spike_strip_is_dodged_only_when_no_tire_was_punctured_on_the_way() {
        let mut c = Challenge::new(Stat::SpikesDodged, 2.0, 3);
        let sites = [[0.0, 0.0], [500.0, 0.0]];
        c.track_spikes(&sites, [20.0, 0.0], 0);
        c.track_spikes(&sites, [-120.0, 0.0], 0);
        assert_eq!(c.spikes_dodged, 1, "the first strip was passed clean");
        c.track_spikes(&sites, [480.0, 0.0], 0);
        c.track_spikes(&sites, [480.0, 200.0], 1);
        assert_eq!(c.spikes_dodged, 1, "a puncture while near the second strip: not dodged");
    }
}
