//! The races the mods see and ask for (`nfsmw.races`): the list, the running race and the results are published each
//! frame, and a start, an abort, the opponents' skill, a win's cash and the races the mods registered are applied here.
//! The rules of the port are `modding/backends/races.rs`; the shared state is `modding/races_feed.rs`.

use std::collections::HashMap;

use blackbox_race::{Course, Gate};
use glam::Vec3;
use nfsmw_data::gameplay::{Placement, Race, RaceParams, RaceType};

use super::{RaceSession, setup};
use crate::modding::races_feed::{self, CUSTOM_BASE, Custom, Now, Outcome, RaceRow, Request, Snapshot};
use crate::scenes::world::{WorldScene, space};

/// The size of the gate a mod's checkpoint makes: through the gate, across the road, up. A mod gives points only, so the
/// gate is the same everywhere **[unconfirmed]**.
const GATE_SIZES: [f32; 3] = [4.0, 30.0, 20.0];

/// What the mods set and the world remembers of the races.
#[derive(Default)]
pub(in crate::scenes::world) struct RaceMods {
    /// The races the mods registered, in order.
    customs: Vec<Custom>,
    /// The opponents' skill and the win's cash a mod set, by race id.
    skill: HashMap<u32, f32>,
    reward: HashMap<u32, u32>,
    /// How the player's races ended this session, by race name.
    outcomes: HashMap<String, Outcome>,
    /// The next race a mod starts is a demo run (`race mod demo`).
    demo: bool,
}

impl RaceMods {
    /// The options a mod's tunings give the race `id`.
    fn options(&self, id: u32, career: bool) -> setup::RaceOptions {
        setup::RaceOptions {
            // The quick race's three difficulty classes take the skill (`docs/specs/race-mode.md` section 11).
            opponents: self.custom(id).map(|custom| custom.opponents as usize),
            difficulty: self.skill.get(&id).map(|skill| (skill.clamp(0.0, 1.0) * 100.0).round() as i32),
            cash: self.reward.get(&id).map(|cash| *cash as f32),
            career,
            demo: self.demo,
            ..Default::default()
        }
    }

    fn custom(&self, id: u32) -> Option<&Custom> {
        self.customs.iter().find(|c| c.id == id)
    }
}

impl WorldScene {
    /// The mods' part of a race frame: applies what they asked for, then publishes the races.
    pub(in crate::scenes::world) fn update_race_mods(&mut self) {
        let seen = self.race_mods.customs.len();
        self.race_mods.customs.extend(races_feed::customs_after(seen));
        for request in races_feed::take_requests() {
            self.apply_race_request(request);
        }
        races_feed::publish(self.races_snapshot());
    }

    /// The console's `race mod`: does what a mod does, to try the interface by hand. It registers a loop of three
    /// checkpoints 150 m apart in front of the car and asks to start it.
    pub(in crate::scenes::world) fn try_mod_race(&mut self, demo: bool) -> Result<String, String> {
        let drive = self.drive.as_ref().ok_or("not driving (use the drive command first)")?;
        let at = drive.position();
        let heading = drive.heading();
        let ahead = |metres: f32| [at.x + heading.cos() * metres, at.y + heading.sin() * metres, at.z];
        let key = format!("console:try{}", self.race_mods.customs.len() + races_feed::customs_after(0).len());
        let custom = Custom {
            id: 0,
            name: key.clone(),
            key,
            kind: RaceType::Sprint as u32,
            laps: 1,
            reward: 1000,
            opponents: if demo { 0 } else { 3 },
            start: [at.x, at.y, at.z],
            heading,
            checkpoints: vec![ahead(150.0), ahead(300.0), ahead(450.0)],
        };
        let id = races_feed::register(custom).ok_or("that race is registered already")?;
        self.race_mods.demo = demo;
        races_feed::request(Request::Start(id));
        Ok(format!("race {id} registered and started as a mod would"))
    }

    fn apply_race_request(&mut self, request: Request) {
        match request {
            Request::Skill(id, skill) => {
                self.race_mods.skill.insert(id, skill);
            }
            Request::Reward(id, cash) => {
                self.race_mods.reward.insert(id, cash);
            }
            Request::Abort => {
                self.abandon_race();
            }
            Request::Start(id) => match self.start_race_by_id(id) {
                Ok(line) => log::info!("race (mod): {line}"),
                Err(why) => log::warn!("race (mod): race {id} did not start: {why}"),
            },
        }
    }

    /// Starts race `id` of the list: a race of the install by its place, or a race a mod registered.
    fn start_race_by_id(&mut self, id: u32) -> Result<String, String> {
        let career = self.career.handle.is_active();
        let options = self.race_mods.options(id, career);
        if id >= CUSTOM_BASE {
            let custom = self.race_mods.custom(id).ok_or("no such race")?.clone();
            let (race, course) = custom_race(&custom);
            return self.begin_race(race, options, Some(course));
        }
        let data = self.gameplay.clone().ok_or("the race data could not be read from the install")?;
        let race = data.races.get(id as usize).ok_or("no such race")?;
        self.begin_race(race.clone(), options, None)
    }

    /// The races as the mods see them now.
    fn races_snapshot(&self) -> Snapshot {
        let mut rows = Vec::new();
        if let Some(data) = self.gameplay.as_ref() {
            rows.extend(
                data.races.iter().enumerate().filter(|(_, r)| setup::runnable(r.race_type)).map(|(i, race)| {
                    row_of(i as u32, race, &self.race_mods.outcomes, self.race_checkpoints(data, race))
                }),
            );
        }
        for custom in &self.race_mods.customs {
            rows.push(RaceRow {
                id: custom.id,
                name: custom.name.clone(),
                kind: custom.kind,
                laps: custom.laps,
                racers: 1,
                start: custom.start,
                checkpoints: custom.checkpoints.clone(),
                done: self.race_mods.outcomes.contains_key(&custom.key),
            });
        }
        let results = rows
            .iter()
            .filter_map(|row| {
                let key = match self.race_mods.custom(row.id) {
                    Some(custom) => custom.key.as_str(),
                    None => row.name.as_str(),
                };
                self.race_mods.outcomes.get(key).map(|outcome| (row.id, *outcome))
            })
            .collect();
        Snapshot { rows, now: self.race_now(), results }
    }

    /// The checkpoints and the finish of a race of the install, render space.
    fn race_checkpoints(&self, data: &nfsmw_data::gameplay::GameplayData, race: &Race) -> Vec<[f32; 3]> {
        let keys = race.checkpoints.iter().copied().chain(race.finish);
        keys.filter_map(|key| data.trigger(key)).map(|t| space::to_render(t.position.to_array()).to_array()).collect()
    }

    /// The race that runs, for the mods.
    fn race_now(&self) -> Option<Now> {
        let session = self.race.as_ref()?;
        let engine = session.engine.as_ref()?;
        let me = engine.racers().first()?;
        let id = self.race_id_of(session)?;
        Some(Now {
            id,
            lap: me.progress.laps + 1,
            checkpoint: me.progress.reached as u32,
            position: engine.places()[0],
            elapsed: engine.elapsed(),
        })
    }

    /// The id a race has on the mods' list.
    fn race_id_of(&self, session: &RaceSession) -> Option<u32> {
        if let Some(custom) = self.race_mods.customs.iter().find(|c| c.key == session.name) {
            return Some(custom.id);
        }
        let data = self.gameplay.as_ref()?;
        data.races.iter().position(|r| r.name == session.name).map(|i| i as u32)
    }

    /// The player's race is over: the mods can read how it went. A race that was left has no result.
    pub(super) fn note_race_outcome(&mut self, session: &RaceSession) {
        let Some(result) = session.result else { return };
        let time = result.time.unwrap_or(0.0);
        let won = result.place == 1 && result.time.is_some();
        let winnings = match won {
            true => session.cash_value.max(0.0).round() as u32,
            false => 0,
        };
        let outcome = Outcome { position: result.place, time, won, winnings };
        self.race_mods.outcomes.insert(session.name.clone(), outcome);
    }
}

fn row_of(id: u32, race: &Race, outcomes: &HashMap<String, Outcome>, checkpoints: Vec<[f32; 3]>) -> RaceRow {
    let start = race.start.map(|p| space::to_render(p.position.to_array()).to_array()).unwrap_or_default();
    RaceRow {
        id,
        name: race.name.clone(),
        kind: race.race_type.map_or(0, |kind| kind as u32),
        laps: race.laps.max(1) as u32,
        racers: race.params.opponents.len() as u32 + 1,
        start,
        checkpoints,
        done: outcomes.contains_key(&race.name),
    }
}

/// The race and the course a mod's definition makes: the checkpoints in order, the last one the finish, each gate turned
/// towards the next point. The stored race has no trigger of its own, so its course is given beside it.
fn custom_race(custom: &Custom) -> (Race, Course) {
    let physics: Vec<Vec3> =
        custom.checkpoints.iter().map(|p| Vec3::from(space::to_physics(glam::Vec3::from(*p)))).collect();
    let start = Vec3::from(space::to_physics(glam::Vec3::from(custom.start)));
    let gates: Vec<Gate> = physics
        .iter()
        .enumerate()
        .map(|(i, at)| {
            let from = if i == 0 { start } else { physics[i - 1] };
            let ahead = physics.get(i + 1).map_or(*at - from, |next| *next - *at);
            let forward = Vec3::new(ahead.x, 0.0, ahead.z).try_normalize().unwrap_or(Vec3::Z);
            Gate::new(*at, forward, Vec3::Y, GATE_SIZES)
        })
        .collect();
    let mut checkpoints = gates;
    let finish = checkpoints.pop();
    let course = Course::new(start, checkpoints, finish, custom.laps.max(1));
    let kind = [
        RaceType::Sprint,
        RaceType::Circuit,
        RaceType::Drag,
        RaceType::Knockout,
        RaceType::Tollbooth,
        RaceType::SpeedTrap,
        RaceType::CheckpointRace,
        RaceType::CashGrab,
        RaceType::Challenge,
        RaceType::SpeedTrapJump,
        RaceType::MilestoneJump,
    ]
    .get(custom.kind as usize)
    .copied()
    .unwrap_or(RaceType::Sprint);
    let race = Race {
        key: custom.id,
        name: custom.key.clone(),
        event: None,
        event_icon: String::new(),
        race_type: Some(kind),
        region: None,
        laps: custom.laps as i32,
        // The heading is radians from +x towards +y, which is the placement's degrees (`spawn_heading` inverse).
        start: Some(Placement { position: start, heading_deg: custom.heading.to_degrees() }),
        finish: None,
        checkpoints: Vec::new(),
        barriers: Vec::new(),
        params: RaceParams { cash_value: custom.reward as f32, do_countdown: true, ..RaceParams::default() },
        story: Default::default(),
    };
    (race, course)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom(points: &[[f32; 3]]) -> Custom {
        Custom {
            id: CUSTOM_BASE,
            key: "m:loop".into(),
            name: "Loop".into(),
            kind: 1,
            laps: 2,
            reward: 750,
            opponents: 2,
            start: [0.0, 0.0, 0.0],
            heading: 0.0,
            checkpoints: points.to_vec(),
        }
    }

    #[test]
    fn a_mods_race_has_its_last_point_as_the_finish_and_its_laps() {
        let (race, course) = custom_race(&custom(&[[100.0, 0.0, 0.0], [200.0, 0.0, 0.0], [300.0, 0.0, 0.0]]));
        assert_eq!(course.checkpoints.len(), 2);
        assert!(course.finish.is_some());
        assert_eq!(course.laps, 2);
        assert_eq!(race.race_type, Some(RaceType::Circuit));
        assert_eq!(race.params.cash_value, 750.0);
        assert!(setup::runnable(race.race_type));
    }

    #[test]
    fn a_single_point_is_the_finish_alone() {
        let (_, course) = custom_race(&custom(&[[100.0, 0.0, 0.0]]));
        assert!(course.checkpoints.is_empty() && course.finish.is_some());
    }

    #[test]
    fn a_car_driving_through_the_points_finishes_the_course() {
        let (_, course) = custom_race(&custom(&[[100.0, 0.0, 0.0], [200.0, 0.0, 0.0]]));
        let laps = course.laps;
        let mut engine =
            blackbox_race::Race::new(course.clone(), blackbox_race::RaceConfig::default(), &[(true, course.start)]);
        engine.start_now();
        let mut at = course.start;
        let mut finished = false;
        for _ in 0..laps {
            for gate in course.checkpoints.iter().chain(course.finish.iter()) {
                let steps = (at.distance(gate.centre) / 4.0).ceil().max(1.0) as u32;
                for step in 1..=steps {
                    let to = at.lerp(gate.centre, step as f32 / steps as f32);
                    finished |=
                        engine.move_racer(0, to).iter().any(|e| matches!(e, blackbox_race::RaceEvent::Finished { .. }));
                }
                at = gate.centre;
            }
        }
        assert!(finished, "{:?}", engine.racers()[0].progress);
    }

    #[test]
    fn the_tunings_become_the_races_options() {
        let mut mods = RaceMods::default();
        mods.skill.insert(3, 0.5);
        mods.reward.insert(3, 900);
        let options = mods.options(3, true);
        assert_eq!((options.difficulty, options.cash, options.career), (Some(50), Some(900.0), true));
        assert_eq!(mods.options(4, false), setup::RaceOptions::default());
    }
}
