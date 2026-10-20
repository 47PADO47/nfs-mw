//! Building a race from the stored data: the course, the grid and the opponents.
//! Specs: `docs/specs/race-mode.md`, `docs/specs/ai-racers.md`, `docs/specs/ai-racers-route.md`.

use blackbox_race::{Course, Gate, RaceConfig, Sphere, Trap};
use glam::Vec3;
use nfsmw_data::gameplay::{BoxTrigger, Character, GameplayData, Placement, Race, RaceType, world_heading};

/// The length of the countdown board, seconds **[unconfirmed]**.
pub const COUNTDOWN_SECS: f32 = 3.0;
/// Metres between two rows of the grid, and between its two columns **[unconfirmed]**.
const ROW_GAP: f32 = 7.5;
const COLUMN_GAP: f32 = 3.5;
/// The quick race's base skills for easy, medium and hard, with the catch-up on and off
/// (`docs/specs/ai-driver-speed-skill.md` section 8.1).
const QUICK_SKILL_CATCH_UP: [f32; 3] = [0.15, 0.40, 1.00];
const QUICK_SKILL_PLAIN: [f32; 3] = [0.15, 0.40, 0.80];

/// What a quick race or a command may change about a race.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RaceOptions {
    pub laps: Option<u32>,
    pub opponents: Option<usize>,
    pub difficulty: Option<i32>,
    pub traffic: Option<i32>,
    /// Count the result in the career.
    pub career: bool,
    /// A demo run: the player's car is put at each gate in turn (`demo.rs`).
    pub demo: bool,
}

/// The gate of a stored box trigger (physics space, y up): its first extent lies along its heading. The stored extents
/// are taken as half sizes **[unconfirmed]**: the first checkpoint of `10.2.1` stands 22 m above the road it is on,
/// which a 40 m high box centred on it (full sizes) would miss, while a 40 m half height holds the road
/// (`docs/specs/world-markers.md` section 3, `docs/formats/gameplay-races.md` open question 1).
pub fn gate_of(trigger: &BoxTrigger) -> Gate {
    let sizes = trigger.dimensions.map(|extent| extent * 2.0);
    Gate::new(trigger.position, world_heading(trigger.heading_deg), Vec3::Y, sizes)
}

/// The course of a race, in physics space. A race with neither a finish line nor a checkpoint has no course.
pub fn course_of(data: &GameplayData, race: &Race, options: &RaceOptions) -> Result<Course, String> {
    let start = race.start.ok_or_else(|| format!("{} has no start", race.name))?;
    let gate = |key: &u32| data.trigger(*key).map(gate_of);
    let checkpoints: Vec<Gate> = race.checkpoints.iter().filter_map(gate).collect();
    let finish = race.finish.as_ref().and_then(gate);
    if checkpoints.is_empty() && finish.is_none() {
        return Err(format!("{} has no checkpoints and no finish line", race.name));
    }
    let mut course = Course::new(start.position, checkpoints, finish, laps_of(race, options));
    // The time bonuses follow the checkpoints that were read, in order (a missing trigger is skipped in both).
    course.bonuses = race.checkpoints.iter().filter_map(|k| data.trigger(*k)).map(|t| t.time_bonus).collect();
    course.traps = race
        .params
        .speed_traps
        .iter()
        .filter_map(|k| data.sphere(*k))
        .map(|s| Trap { zone: Sphere { centre: s.position, radius: s.radius }, threshold_kmh: s.threshold_speed })
        .collect();
    Ok(course)
}

/// The laps of the run: the option, else the race's, at least one.
pub fn laps_of(race: &Race, options: &RaceOptions) -> u32 {
    let stored = u32::try_from(race.laps).unwrap_or(1);
    options.laps.unwrap_or(stored).max(1)
}

/// The rules of the run from the stored parameters and the number of speed traps.
pub fn config_of(race: &Race) -> RaceConfig {
    let p = &race.params;
    RaceConfig {
        knockouts_per_lap: u32::try_from(p.knockouts_per_lap).unwrap_or(0),
        rank_by_points: p.rank_by_points,
        time_limit: p.time_limit,
        countdown: if p.do_countdown && !p.rolling_start { COUNTDOWN_SECS } else { 0.0 },
        speed_traps: p.speed_traps.len(),
        overtime_penalty: p.overtime_penalty,
    }
}

/// The grid slot `index` (0 is the player's): two columns, a row behind per two cars.
pub fn grid_slot(start: &Placement, index: usize) -> Placement {
    let forward = world_heading(start.heading_deg);
    let side = Vec3::Y.cross(forward);
    let (row, column) = ((index / 2) as f32, (index % 2) as f32);
    Placement {
        position: start.position - forward * row * ROW_GAP + side * column * COLUMN_GAP,
        heading_deg: start.heading_deg,
    }
}

/// The heading of a placement as the world's spawn heading: radians from +X in render space.
pub fn spawn_heading(heading_deg: f32) -> f32 {
    let forward = world_heading(heading_deg);
    // Physics (x, y, z) is render (z, -x, y).
    (-forward.x).atan2(forward.z)
}

/// One opponent of the run.
#[derive(Debug, Clone, PartialEq)]
pub struct Opponent {
    pub name: String,
    /// The base skill, 0 to 1.
    pub skill: f32,
    /// `MinimumAIPerformance`.
    pub bias: f32,
    /// The car the character names, if it does: a `pvehicle` name.
    pub car: Option<String>,
    /// The marker this racer starts at instead of the grid.
    pub start: Option<Placement>,
}

/// The base skill of a character: the quick race's table by difficulty, or the character's own level in the career.
pub fn base_skill(character: &Character, quick: bool, difficulty_class: usize, catch_up: bool) -> f32 {
    if quick {
        let table = if catch_up { QUICK_SKILL_CATCH_UP } else { QUICK_SKILL_PLAIN };
        return table[difficulty_class.min(2)];
    }
    (character.skill_level as f32 / 100.0).clamp(0.0, 1.0)
}

/// The opponents of a run. A race lists its own; `options.opponents` sets the count (cycling the list, or using the
/// generic `character_smart` for a race that lists none). A quick race (`!options.career`) uses the quick table.
pub fn opponents_of(
    data: &GameplayData,
    race: &Race,
    options: &RaceOptions,
    resolve_preset: &dyn Fn(&str) -> Option<String>,
) -> Vec<Opponent> {
    let listed: Vec<&Character> = race.params.opponents.iter().filter_map(|k| data.character(*k)).collect();
    let generic = data.characters.iter().find(|c| c.name.rsplit('/').next() == Some("character_smart"));
    let pool: Vec<&Character> = match (listed.is_empty(), generic) {
        (false, _) => listed,
        (true, Some(generic)) => vec![generic],
        (true, None) => Vec::new(),
    };
    let wanted = match (options.opponents, race.params.opponents.is_empty()) {
        (Some(n), _) => n,
        (None, true) => 0,
        (None, false) => pool.len(),
    };
    let difficulty = options.difficulty.unwrap_or(race.params.difficulty);
    let class = nfsmw_data::gameplay::RaceParams { difficulty, ..race.params.clone() }.difficulty_class();
    // A boss in a quick race is the generic character, as the original does (`docs/formats/gameplay-races.md` section 5).
    let quick = !options.career;
    (0..wanted)
        .filter_map(|i| {
            let character = *pool.get(i % pool.len().max(1))?;
            let character = match (quick, character.is_boss, generic) {
                (true, true, Some(generic)) => generic,
                _ => character,
            };
            let car = character.preset_ride.as_deref().and_then(resolve_preset).or_else(|| character.car_type.clone());
            Some(Opponent {
                name: character.name.clone(),
                skill: base_skill(character, quick, class, race.params.catch_up),
                bias: character.min_performance,
                car,
                start: character.force_start,
            })
        })
        .collect()
}

/// A stock car for an opponent that names none: a fixed pick from `pool` that depends on the race and the racer, so the
/// same race always gives the same cars **[unconfirmed]**.
pub fn stock_car(pool: &[String], race_key: u32, racer: usize) -> Option<&String> {
    if pool.is_empty() {
        return None;
    }
    let mix = race_key.wrapping_mul(0x9E37_79B1).wrapping_add((racer as u32).wrapping_mul(0x85EB_CA6B));
    pool.get((mix >> 8) as usize % pool.len())
}

/// The catch-up offset as the driver applies it: a positive glue is scaled by `Table[0.33, 0.66, 1.0](base_skill)`, a
/// negative one by `Table[1.0, 1.0, 0.66](base_skill)`, and both are halved in a speed-trap race
/// (`docs/specs/ai-driver-speed-skill.md` section 8.2).
pub fn scale_glue(glue: f32, base_skill: f32, speed_trap: bool) -> f32 {
    let table = if glue >= 0.0 { [0.33, 0.66, 1.0] } else { [1.0, 1.0, 0.66] };
    let at = base_skill.clamp(0.0, 1.0) * 2.0;
    let low = (at.floor() as usize).min(1);
    let scale = table[low] + (table[low + 1] - table[low]) * (at - low as f32);
    glue * scale * if speed_trap { 0.5 } else { 1.0 }
}

/// Whether the race type is one this world can run today.
pub fn runnable(kind: Option<RaceType>) -> bool {
    matches!(
        kind,
        Some(
            RaceType::Sprint
                | RaceType::Circuit
                | RaceType::Drag
                | RaceType::Knockout
                | RaceType::Tollbooth
                | RaceType::SpeedTrap
                | RaceType::Challenge
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsmw_data::gameplay::RaceParams;

    fn trigger(key: u32, at: Vec3) -> BoxTrigger {
        BoxTrigger {
            key,
            name: format!("t{key}"),
            position: at,
            heading_deg: 0.0,
            dimensions: [2.0, 40.0, 40.0],
            one_shot: false,
            fire_on_exit: false,
            particle_effect: None,
            flare_spacing: None,
            time_bonus: 0.0,
            resets_player: false,
        }
    }

    fn character(key: u32, name: &str, skill: i32, boss: bool) -> Character {
        Character {
            key,
            name: name.to_owned(),
            skill_level: skill,
            min_performance: 0.5,
            preset_ride: None,
            car_type: None,
            is_boss: boss,
            force_start: None,
        }
    }

    fn race(opponents: Vec<u32>) -> Race {
        Race {
            key: 1,
            name: "race_bin_1/1_1_1_sprint".into(),
            event: None,
            event_icon: "p2p".into(),
            race_type: Some(RaceType::Sprint),
            region: None,
            laps: 1,
            start: Some(Placement { position: Vec3::new(0.0, 0.0, 0.0), heading_deg: 0.0 }),
            finish: Some(10),
            checkpoints: vec![11, 12],
            barriers: Vec::new(),
            params: RaceParams { opponents, ..Default::default() },
        }
    }

    fn data() -> GameplayData {
        GameplayData {
            triggers: vec![
                trigger(10, Vec3::new(0.0, 0.0, 300.0)),
                trigger(11, Vec3::new(0.0, 0.0, 100.0)),
                trigger(12, Vec3::new(0.0, 0.0, 200.0)),
            ],
            characters: vec![
                character(20, "race_bin_1/character1", 80, false),
                character(21, "race_bin_1/boss", 90, true),
                character(22, "character_smart", 0, false),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn the_course_has_the_checkpoints_in_order_and_the_finish() {
        let course = course_of(&data(), &race(Vec::new()), &RaceOptions::default()).unwrap();
        assert_eq!(course.checkpoints.len(), 2);
        assert_eq!(course.checkpoints[0].centre.z, 100.0);
        assert_eq!(course.finish.unwrap().centre.z, 300.0);
        assert_eq!(course.laps, 1);
    }

    #[test]
    fn the_stored_extents_are_half_sizes() {
        // A 2 x 40 x 40 trigger holds a point 30 m up and 39 m across, and 1.9 m along its heading.
        let gate = gate_of(&trigger(1, Vec3::ZERO));
        assert!(gate.contains(Vec3::new(39.0, 30.0, 1.9)));
        assert!(!gate.contains(Vec3::new(0.0, 0.0, 2.1)));
    }

    #[test]
    fn the_course_carries_the_time_bonuses_and_the_speed_traps() {
        let mut d = data();
        d.triggers[1].time_bonus = 25.0;
        d.spheres = vec![nfsmw_data::gameplay::SphereTrigger {
            key: 50,
            name: "trap".into(),
            position: Vec3::new(1.0, 2.0, 3.0),
            radius: 20.0,
            threshold_speed: 88.0,
        }];
        let mut r = race(Vec::new());
        r.params.speed_traps = vec![50, 51];
        let course = course_of(&d, &r, &RaceOptions::default()).unwrap();
        assert_eq!(course.bonus(1), 25.0);
        assert_eq!(course.bonus(2), 0.0);
        assert_eq!(course.traps.len(), 1, "a trap that was not read is skipped");
        assert_eq!(course.traps[0].threshold_kmh, 88.0);
        assert_eq!(config_of(&r).speed_traps, 2);
    }

    #[test]
    fn a_race_without_gates_has_no_course() {
        let mut empty = race(Vec::new());
        empty.finish = None;
        empty.checkpoints.clear();
        assert!(course_of(&data(), &empty, &RaceOptions::default()).is_err());
    }

    #[test]
    fn the_lap_option_replaces_the_stored_laps() {
        let options = RaceOptions { laps: Some(4), ..Default::default() };
        assert_eq!(laps_of(&race(Vec::new()), &options), 4);
        assert_eq!(laps_of(&race(Vec::new()), &RaceOptions::default()), 1);
    }

    #[test]
    fn a_rolling_start_has_no_countdown() {
        let mut r = race(Vec::new());
        assert_eq!(config_of(&r).countdown, COUNTDOWN_SECS);
        r.params.rolling_start = true;
        assert_eq!(config_of(&r).countdown, 0.0);
    }

    #[test]
    fn the_grid_has_two_columns_behind_the_start() {
        let start = Placement { position: Vec3::new(5.0, 1.0, 5.0), heading_deg: 0.0 };
        let player = grid_slot(&start, 0);
        assert_eq!(player.position, start.position);
        let beside = grid_slot(&start, 1);
        assert!((beside.position.z - 5.0).abs() < 1e-4 && (beside.position.x - 5.0).abs() > 3.0);
        let behind = grid_slot(&start, 2);
        assert!((behind.position.z - (5.0 - 7.5)).abs() < 1e-4, "{behind:?}");
        assert_eq!(behind.position.x, 5.0);
    }

    #[test]
    fn the_spawn_heading_points_along_the_grid() {
        // Heading 0 is +z in physics, which is +x in render space: radians 0.
        assert!(spawn_heading(0.0).abs() < 1e-5);
        // Heading 90 is -x in physics, which is +y in render space.
        assert!((spawn_heading(90.0) - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    }

    #[test]
    fn a_listed_race_uses_its_own_characters_with_their_skill_in_a_career() {
        let r = race(vec![20, 21]);
        let career = RaceOptions { career: true, ..Default::default() };
        let opponents = opponents_of(&data(), &r, &career, &|_| None);
        assert_eq!(opponents.len(), 2);
        assert_eq!(opponents[0].skill, 0.8);
        assert_eq!(opponents[1].skill, 0.9);
    }

    #[test]
    fn a_quick_race_replaces_a_boss_and_uses_the_difficulty_table() {
        let r = race(vec![21]);
        let quick = RaceOptions { difficulty: Some(33), ..Default::default() };
        let opponents = opponents_of(&data(), &r, &quick, &|_| None);
        assert_eq!(opponents.len(), 1);
        assert_eq!(opponents[0].name, "character_smart");
        assert_eq!(opponents[0].skill, 0.15);
        let hard = RaceOptions { difficulty: Some(100), ..Default::default() };
        assert_eq!(opponents_of(&data(), &r, &hard, &|_| None)[0].skill, 1.0);
    }

    #[test]
    fn a_race_with_no_opponents_gets_the_asked_count_of_generic_ones() {
        let r = race(Vec::new());
        assert!(opponents_of(&data(), &r, &RaceOptions::default(), &|_| None).is_empty());
        let three = RaceOptions { opponents: Some(3), ..Default::default() };
        assert_eq!(opponents_of(&data(), &r, &three, &|_| None).len(), 3);
    }

    #[test]
    fn a_preset_ride_resolves_to_its_car() {
        let mut d = data();
        d.characters[0].preset_ride = Some("BL5".into());
        let r = race(vec![20]);
        let opponents = opponents_of(&d, &r, &RaceOptions { career: true, ..Default::default() }, &|name| {
            (name == "BL5").then(|| "mustanggt".to_owned())
        });
        assert_eq!(opponents[0].car.as_deref(), Some("mustanggt"));
    }

    #[test]
    fn the_stock_pick_is_stable_and_differs_between_racers() {
        let pool: Vec<String> = (0..20).map(|i| format!("car{i}")).collect();
        assert_eq!(stock_car(&pool, 7, 1), stock_car(&pool, 7, 1));
        let picks: std::collections::HashSet<_> = (0..6).filter_map(|i| stock_car(&pool, 7, i)).collect();
        assert!(picks.len() > 2, "{picks:?}");
        assert_eq!(stock_car(&[], 1, 1), None);
    }

    #[test]
    fn a_positive_glue_is_scaled_down_for_a_weak_driver_and_a_negative_one_for_a_strong_one() {
        assert!((scale_glue(1.0, 0.0, false) - 0.33).abs() < 1e-5);
        assert!((scale_glue(1.0, 0.5, false) - 0.66).abs() < 1e-5);
        assert!((scale_glue(1.0, 1.0, false) - 1.0).abs() < 1e-5);
        assert!((scale_glue(-1.0, 0.0, false) + 1.0).abs() < 1e-5);
        assert!((scale_glue(-1.0, 1.0, false) + 0.66).abs() < 1e-5);
        assert!((scale_glue(1.0, 1.0, true) - 0.5).abs() < 1e-5, "halved in a speed-trap race");
    }

    #[test]
    fn only_the_types_the_world_can_run_are_runnable() {
        assert!(runnable(Some(RaceType::Drag)));
        assert!(!runnable(Some(RaceType::CashGrab)));
        assert!(!runnable(None));
    }

    /// The side bins 18 to 21 (tollbooth challenges, the challenge series, the collectors; the OPM test bin 20 is a developer bin nothing enters): every race
    /// of them is one of the runnable types, and has a course or is a pursuit challenge with a statistic to reach.
    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn every_race_of_the_side_bins_can_be_started() {
        let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
        let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let data = nfsmw_data::gameplay::load(&dir).unwrap();
        let side: Vec<_> = data
            .races
            .iter()
            .filter(|r| ["18.", "19.", "21."].iter().any(|bin| crate::career::race_event_id(r).starts_with(bin)))
            .collect();
        assert!(side.len() > 50, "{} side bin races", side.len());
        for race in side {
            assert!(runnable(race.race_type), "{} is {:?}", race.name, race.race_type);
            let challenge = race.race_type == Some(RaceType::Challenge);
            let ok = match challenge {
                true => {
                    race.params.milestone.is_some()
                        && crate::scenes::world::race::challenge::Stat::of(race.params.milestone.as_deref().unwrap())
                            .is_some()
                }
                false => course_of(&data, race, &RaceOptions::default()).is_ok(),
            };
            assert!(ok, "{} cannot start", race.name);
        }
    }

    /// Walks a straight line through every gate of every race of the install and checks that the race ends with the
    /// walker first, with the laps counted.
    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn every_install_course_can_be_finished_by_walking_through_its_gates() {
        let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
        let dir = game_install::GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let data = nfsmw_data::gameplay::load(&dir).unwrap();
        let (mut ran, mut skipped) = (0, 0);
        for race in data.races.iter().filter(|r| runnable(r.race_type)) {
            let options = RaceOptions::default();
            let Ok(course) = course_of(&data, race, &options) else {
                skipped += 1;
                continue;
            };
            let start = course.start;
            let mut engine =
                blackbox_race::Race::new(course.clone(), config_of(race), &[(true, start), (false, start)]);
            engine.start_now();
            let mut at = start;
            let mut finished = false;
            let mut gates: Vec<Vec3> = course.checkpoints.iter().map(|g| g.centre).collect();
            gates.extend(course.finish.iter().map(|g| g.centre));
            for _ in 0..course.laps {
                for centre in &gates {
                    let steps = (at.distance(*centre) / 4.0).ceil().max(1.0) as u32;
                    for step in 1..=steps {
                        let to = at.lerp(*centre, step as f32 / steps as f32);
                        finished |= engine
                            .move_racer(0, to)
                            .iter()
                            .any(|e| matches!(e, blackbox_race::RaceEvent::Finished { .. }));
                    }
                    at = *centre;
                }
            }
            assert!(finished, "{} was not finished: {:?}", race.name, engine.racers()[0].progress);
            assert_eq!(engine.places()[0], 1, "{}", race.name);
            ran += 1;
        }
        eprintln!("{ran} courses finished, {skipped} races without a course");
        assert!(ran > 200, "{ran}");
    }
}
