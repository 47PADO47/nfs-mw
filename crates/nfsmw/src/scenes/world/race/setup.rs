//! Building a race from the stored data: the course, the grid and the opponents.
//! Specs: `docs/specs/race-mode.md`, `docs/specs/ai-racers.md`, `docs/specs/ai-racers-route.md`.

use blackbox_race::{Course, Gate, RaceConfig, Sphere, Trap};
use glam::Vec3;
use nfsmw_data::gameplay::{BoxTrigger, Character, GameplayData, Placement, Race, RaceType, world_heading};

use super::names;

/// The length of the countdown board, seconds **[unconfirmed]**.
pub const COUNTDOWN_SECS: f32 = 3.0;
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
    /// The catch-up (rubber band) switch, instead of the race's own (`catch_up` of the data).
    pub catch_up: Option<bool>,
    /// The heat level of the race's police, 0 (none) to 3 (maximum), when the player chose one (custom race).
    pub heat: Option<u8>,
    /// The cash a win pays, instead of the race's own (`nfsmw.races`' `set_reward`).
    pub cash: Option<f32>,
    /// Count the result in the career.
    pub career: bool,
    /// A demo run: the player's car is put at each gate in turn (`demo.rs`).
    pub demo: bool,
}

/// The gate of a stored box trigger (physics space, y up): its first extent lies along its heading, the second across.
/// The two horizontal extents are full sizes and the third is the half height **[exe]**: the trigger constructor builds
/// the box from `b` across, `a` along and `c` above and below. The half height is also what the first checkpoint of
/// `10.2.1` needs, as it stands 22 m above the road (`docs/specs/race-wrongway.md` section 2, `docs/specs/world-markers.md`
/// section 3).
pub fn gate_of(trigger: &BoxTrigger) -> Gate {
    let [along, across, half_height] = trigger.dimensions;
    Gate::new(trigger.position, world_heading(trigger.heading_deg), Vec3::Y, [along, across, half_height * 2.0])
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
        max_move: Some(super::route::MAX_MOVE),
    }
}

/// One opponent of the run.
#[derive(Debug, Clone, PartialEq)]
pub struct Opponent {
    pub name: String,
    /// The language label of the name the data fixes for this racer in a career race (`names::fixed_label`).
    pub name_label: Option<String>,
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
    // A boss race names its racer after the Blacklist bin it belongs to (`docs/specs/racer-names.md` section 2).
    let boss_bin = race.story.boss_race.then(|| names::bin_of(&race.name)).flatten();
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
                name_label: names::fixed_label(options.career, boss_bin, character.racer_name.as_deref()),
                skill: base_skill(character, quick, class, options.catch_up.unwrap_or(race.params.catch_up)),
                bias: match quick {
                    true => 0.0,
                    false => character.min_performance,
                },
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
#[path = "setup_tests.rs"]
mod tests;
