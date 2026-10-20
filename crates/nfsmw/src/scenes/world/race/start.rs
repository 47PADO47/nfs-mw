//! Starting a race: choosing the stored race, planning the grid and the opponents, and asking for the player's car
//! to stand on the grid. The rest of the start happens in `load.rs` once the world and the models are ready.

use blackbox_race::{CatchUpGains, Course, RaceConfig};
use nfsmw_data::gameplay::{GameplayData, Race, RaceType};

use super::setup::{self, RaceOptions};
use super::{Banner, RaceSession, RacerPlan, Stage};
use crate::career::event_id;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::{View, WorldScene, space};

/// What the run needs once the grid is ready.
pub(in crate::scenes::world) struct Prepared {
    pub course: Course,
    pub config: RaceConfig,
    /// A rolling start: no hold and no countdown; the cars begin moving.
    pub rolling: bool,
    /// The start speed of a rolling start, m/s.
    pub initial_speed: f32,
    pub catch_up: bool,
    pub gains: CatchUpGains,
}

/// The race a query names: the exact collection name, then a unique name ending in `/query`, then a unique career event
/// id (`1.5.2`), then a unique name containing the query.
pub(super) fn resolve<'a>(data: &'a GameplayData, query: &str) -> Result<&'a Race, String> {
    let query = query.trim();
    if let Some(race) = data.race_named(query) {
        return Ok(race);
    }
    let lower = query.to_ascii_lowercase();
    let matches = |test: &dyn Fn(&str) -> bool| -> Vec<&Race> {
        data.races.iter().filter(|r| test(&r.name.to_ascii_lowercase())).collect()
    };
    let by_leaf = matches(&|name| name.rsplit('/').next() == Some(lower.as_str()));
    let by_event = data.races.iter().filter(|r| event_id(&r.name) == query).collect::<Vec<_>>();
    let by_part = matches(&|name| name.contains(&lower));
    for found in [by_leaf, by_event, by_part] {
        match found.as_slice() {
            [] => continue,
            [only] => return Ok(only),
            many => {
                let names: Vec<&str> = many.iter().take(6).map(|r| r.name.as_str()).collect();
                return Err(format!("{query:?} matches {} races, for example {}", many.len(), names.join(", ")));
            }
        }
    }
    Err(format!("no race matches {query:?} (race list shows them)"))
}

impl WorldScene {
    /// Asks for a race to start as soon as the player's car exists: the front end uses it for a quick race, which opens
    /// the world and the race in one go.
    pub fn queue_race(&mut self, query: String, options: RaceOptions) {
        self.queued_race = Some((query, options));
    }

    /// Starts the race `query` names. The player's car is put on the grid and the opponents are loaded; the countdown
    /// starts when everything stands. Quick race and the console use this; the career's race entry too.
    pub(in crate::scenes::world) fn start_race(&mut self, query: &str, options: RaceOptions) -> Result<String, String> {
        if self.race.is_some() {
            return Err("a race is already running (race stop ends it)".into());
        }
        let data = self.gameplay.clone().ok_or("the race data could not be read from the install")?;
        let race = resolve(&data, query)?.clone();
        if !setup::runnable(race.race_type) {
            return Err(format!("{} is a {:?} race, which this version does not run", race.name, race.race_type));
        }
        if self.drive.is_none() {
            return Err("not driving (use the drive command first)".into());
        }
        let course = setup::course_of(&data, &race, &options)?;
        let start = race.start.ok_or("the race has no start")?;
        let tables = nfsmw_data::car::CarTables::load(&self.dir).ok();
        let resolve_preset = |name: &str| tables.as_ref().and_then(|t| t.preset(name)).map(|p| p.car_type_name.clone());
        let opponents = setup::opponents_of(&data, &race, &options, &resolve_preset);
        let pool = stock_pool(&self.dir);
        let has_road = self.traffic.is_some();
        let plan = plan_racers(&race, &start, &opponents, &pool, has_road);
        let p = &race.params;
        let prepared = Prepared {
            course,
            config: setup::config_of(&race),
            rolling: p.rolling_start,
            initial_speed: p.initial_speed.max(0.0),
            catch_up: p.catch_up,
            gains: CatchUpGains::default(),
        };
        let prepared_catch_up = prepared.catch_up;
        let speed = if prepared.rolling { prepared.initial_speed } else { 0.0 };
        let player_spawn = plan[0].spawn;
        if let Some(drive) = self.drive.as_mut() {
            drive.place_exactly(player_spawn, speed);
            drive.hold_on_grid(!prepared.rolling);
        }
        self.view = View::Chase;
        let previous_traffic = self.traffic_manual;
        // `ForceTrafficDensity` above 0 sets the density at the start (`docs/specs/race-mode.md` section 2).
        let stored = if p.force_traffic_density > 0 { p.force_traffic_density } else { p.traffic_level };
        let level = options.traffic.unwrap_or(stored).clamp(0, 100);
        let drag = race.race_type == Some(RaceType::Drag);
        let cars = match drag {
            true => 0,
            false => (self.traffic_setting.cars * level as u32 / 100) as usize,
        };
        self.traffic_manual = Some(cars);
        if let Some(traffic) = self.traffic.as_mut()
            && !p.cops_in_race
        {
            traffic.stop_pursuit();
        }
        log::info!(
            "race: {} ({:?}), {} lap(s), {} opponent(s), traffic {} car(s)",
            race.name,
            race.race_type,
            prepared.course.laps,
            plan.len() - 1,
            cars
        );
        let racers = plan.len();
        self.race = Some(RaceSession {
            event: event_id(&race.name),
            name: race.name.clone(),
            kind: race.race_type.unwrap_or(RaceType::Sprint),
            options,
            stage: Stage::Loading { waited: 0.0 },
            engine: None,
            plan,
            setup: Some(prepared),
            previous_traffic,
            glue: Vec::new(),
            catch_up: prepared_catch_up,
            glue_clock: 0.0,
            demo_clock: 0.0,
            lane: Default::default(),
            emu: Vec::new(),
            log_clock: 0.0,
            banner: Banner::default(),
            result: None,
            report: None,
            cash_value: p.cash_value,
            pool,
        });
        Ok(format!("{}: {} racer(s), getting to the grid", race.name, racers))
    }
}

/// The stock cars an opponent may drive when its character names none: the cars of the car lot.
fn stock_pool(dir: &game_install::GameDir) -> Vec<String> {
    let folders = nfsmw_data::car::list(dir);
    let on_disk = |name: &String| folders.iter().any(|f| f.eq_ignore_ascii_case(name));
    nfsmw_data::career_cars::load(dir)
        .map(|cars| cars.into_iter().filter(|c| c.for_sale).filter_map(|c| c.name).filter(on_disk).collect())
        .unwrap_or_default()
}

/// The racers' plans: the player first, then the opponents on the grid. Without a road network there are no opponents.
fn plan_racers(
    race: &Race,
    start: &nfsmw_data::gameplay::Placement,
    opponents: &[setup::Opponent],
    pool: &[String],
    has_road: bool,
) -> Vec<RacerPlan> {
    let slot = |placement: &nfsmw_data::gameplay::Placement| Spawn {
        position: space::to_render(placement.position.to_array()),
        heading: setup::spawn_heading(placement.heading_deg),
    };
    let mut plan = vec![RacerPlan {
        name: "player".into(),
        model: None,
        spawn: slot(start),
        skill: 1.0,
        bias: 1.0,
        tried: Vec::new(),
    }];
    if !has_road {
        return plan;
    }
    for (i, opponent) in opponents.iter().enumerate() {
        let placement = opponent.start.unwrap_or_else(|| setup::grid_slot(start, i + 1));
        let model = opponent.car.clone().or_else(|| setup::stock_car(pool, race.key, i).cloned());
        let Some(model) = model else { continue };
        plan.push(RacerPlan {
            name: opponent.name.clone(),
            model: Some(model),
            spawn: slot(&placement),
            skill: opponent.skill,
            bias: opponent.bias,
            tried: Vec::new(),
        });
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsmw_data::gameplay::RaceParams;

    fn race(name: &str) -> Race {
        Race {
            key: name.len() as u32,
            name: name.to_owned(),
            event_icon: "p2p".into(),
            race_type: Some(RaceType::Sprint),
            region: None,
            laps: 1,
            start: None,
            finish: None,
            checkpoints: Vec::new(),
            barriers: Vec::new(),
            params: RaceParams::default(),
        }
    }

    fn data() -> GameplayData {
        GameplayData {
            races: vec![
                race("race_bin_10/10_2_1_sprint"),
                race("race_bin_10/10_2_2_sprint"),
                race("race_bin_09/9_1_1_circuit"),
                race("race_bin_09/9_1_1_circuit_r"),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn the_exact_name_picks_a_race() {
        let d = data();
        assert_eq!(resolve(&d, "race_bin_10/10_2_2_sprint").unwrap().name, "race_bin_10/10_2_2_sprint");
    }

    #[test]
    fn a_leaf_name_or_an_event_id_picks_a_race() {
        let d = data();
        assert_eq!(resolve(&d, "10_2_1_sprint").unwrap().name, "race_bin_10/10_2_1_sprint");
        assert_eq!(resolve(&d, "10.2.2").unwrap().name, "race_bin_10/10_2_2_sprint");
    }

    #[test]
    fn a_query_that_matches_many_names_some() {
        let err = resolve(&data(), "sprint").unwrap_err();
        assert!(err.contains("matches 2 races"), "{err}");
    }

    #[test]
    fn an_unknown_query_is_an_error() {
        assert!(resolve(&data(), "nothing").is_err());
    }

    #[test]
    fn a_unique_part_of_a_name_picks_a_race() {
        assert_eq!(resolve(&data(), "9_1_1_circuit_r").unwrap().name, "race_bin_09/9_1_1_circuit_r");
    }

    #[test]
    fn the_plan_puts_the_player_first_and_skips_cars_that_cannot_be_named() {
        let start = nfsmw_data::gameplay::Placement { position: glam::Vec3::ZERO, heading_deg: 0.0 };
        let named = setup::Opponent { name: "a".into(), skill: 0.5, bias: 0.5, car: Some("c1".into()), start: None };
        let nameless = setup::Opponent { name: "b".into(), skill: 0.5, bias: 0.5, car: None, start: None };
        let plan = plan_racers(&race("r"), &start, &[named.clone(), nameless.clone()], &[], true);
        assert_eq!(plan.len(), 2, "the car-less opponent has no pool to draw from");
        assert!(plan[0].model.is_none());
        assert_eq!(plan[1].model.as_deref(), Some("c1"));
        let pool = vec!["x".to_owned()];
        let plan = plan_racers(&race("r"), &start, &[named, nameless], &pool, true);
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[2].model.as_deref(), Some("x"));
    }

    #[test]
    fn without_a_road_network_nobody_else_races() {
        let start = nfsmw_data::gameplay::Placement { position: glam::Vec3::ZERO, heading_deg: 0.0 };
        let named = setup::Opponent { name: "a".into(), skill: 0.5, bias: 0.5, car: Some("c1".into()), start: None };
        assert_eq!(plan_racers(&race("r"), &start, &[named], &[], false).len(), 1);
    }
}
