//! The presentation of the prologue, step by step, for checks and the `nis-report` command: for each prologue event the
//! intro and outro steps the world runs (message cards, movies, NIS scenes, from `intro.rs` and `outro.rs`), each scene
//! run headless with the racers its car channels take (`nis/report.rs`). Nothing here needs a window, a GPU or sound.

use game_install::GameDir;
use nfsmw_career::story::prologue::PROLOGUE_CHAIN;
use nfsmw_career::story::{IntroStep, OutroStep, outro_steps};
use nfsmw_data::gameplay::{GameplayData, Race};
use serde::Serialize;

use super::intro::{MESSAGE_LABEL, intro_plan, story_fields};
use super::setup::{self, RaceOptions};
use crate::career::race_event_id;
use crate::scenes::world::nis::NisKind;
use crate::scenes::world::nis::report::{self, SceneReport, Seat, Seating};

/// One step of an event's intro or outro, as the world runs it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepReport {
    /// `message`, `movie` or `nis`.
    pub step: String,
    /// The message id, the movie or the scene, with what the world shows for it.
    pub detail: String,
}

/// The presentation of one prologue event.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventReport {
    pub event: String,
    pub race: String,
    /// The car the race forces on the player.
    pub player_car: Option<String>,
    pub steps: Vec<StepReport>,
    pub scenes: Vec<SceneReport>,
}

/// The presentation of the six prologue events, in the order the chain plays them (`story-flow.md` section 2.1).
pub fn prologue(dir: &GameDir) -> Result<Vec<EventReport>, String> {
    let data = nfsmw_data::gameplay::cached(dir).map_err(|e| format!("{e:#}"))?;
    PROLOGUE_CHAIN.iter().map(|event| event_report(dir, &data, event)).collect()
}

/// One scene run headless. With `event`, the scene stands as that event's intro (or outro) would play it, with the
/// event's racers and camera track; without, as the console's `nis play` does: the player on `car1`, track 0.
pub fn scene(dir: &GameDir, name: &str, event: Option<&str>) -> Result<SceneReport, String> {
    let kind = NisKind::of_scene(name);
    let Some(event) = event else { return report::run(dir, name, kind, 0, &Seating::default()) };
    let data = nfsmw_data::gameplay::cached(dir).map_err(|e| format!("{e:#}"))?;
    let race = race_of(&data, event)?;
    let intro = intro_plan(&race.story, true)
        .into_iter()
        .any(|step| matches!(step, IntroStep::Nis { scene, .. } if scene.eq_ignore_ascii_case(name)));
    match intro {
        true => report::run(dir, name, kind, track_of(race), &seating(dir, &data, race)),
        false => report::run(dir, name, kind, 0, &Seating::default()),
    }
}

/// The `nis-report` command's answer as JSON: the scene `scene` run headless (as `event`'s intro when given), or the whole
/// prologue when no scene is named.
pub(crate) fn report_json(dir: &GameDir, scene_name: Option<&str>, event: Option<&str>) -> Result<String, String> {
    let json = match scene_name {
        Some(name) => serde_json::to_string_pretty(&scene(dir, name, event)?),
        None => serde_json::to_string_pretty(&prologue(dir)?),
    };
    json.map_err(|e| e.to_string())
}

fn race_of<'a>(data: &'a GameplayData, event: &str) -> Result<&'a Race, String> {
    data.races.iter().find(|race| race_event_id(race) == event).ok_or_else(|| format!("no race of event {event}"))
}

fn track_of(race: &Race) -> u32 {
    u32::try_from(race.story.intro_camera_track).unwrap_or(0)
}

fn event_report(dir: &GameDir, data: &GameplayData, event: &str) -> Result<EventReport, String> {
    let race = race_of(data, event)?;
    let english = nfsmw_data::languages::table(dir, nfsmw_data::languages::ENGLISH_FILE).ok();
    let text = |label: u32| english.as_ref().and_then(|table| table.get(label)).unwrap_or_default();
    let mut steps = Vec::new();
    let mut scenes = Vec::new();
    for step in intro_plan(&race.story, true) {
        match step {
            IntroStep::Tutorial => {}
            IntroStep::WarpToStart => steps.push(step_of("warp", "the player is put on the start".into())),
            IntroStep::Message(id) => {
                let shown = text(MESSAGE_LABEL).replace('^', " / ");
                let shown = if shown.is_empty() { crate::hud::CARD_TEXT.to_owned() } else { shown };
                let detail = format!("IntroMessageID {id}: SixDaysLater.fng, {MESSAGE_LABEL:#010X} \"{shown}\"");
                steps.push(step_of("message", detail));
            }
            IntroStep::Movie(movie) => steps.push(step_of("movie", movie)),
            IntroStep::Nis { scene, movie } => {
                if let Some(movie) = movie {
                    steps.push(step_of("movie", format!("{movie} (the NIS's movie argument)")));
                }
                steps.push(step_of("nis", format!("{scene} track {} at the race start", track_of(race))));
                let kind = NisKind::of_scene(&scene);
                scenes.push(report::run(dir, &scene, kind, track_of(race), &seating(dir, data, race))?);
            }
        }
    }
    for step in outro_steps(&story_fields(&race.story), race.story.has_outro_marker) {
        match step {
            OutroStep::Nis(scene) => {
                steps.push(step_of("nis", format!("{scene} track 0 where the player's car stands (outro)")));
                scenes.push(report::run(dir, &scene, NisKind::of_scene(&scene), 0, &Seating::default())?);
            }
            OutroStep::Message(id) => steps.push(step_of("message", format!("OutroMessageID {id}"))),
            OutroStep::Movie(movie) => steps.push(step_of("movie", format!("{movie} (outro)"))),
        }
    }
    Ok(EventReport {
        event: event.to_owned(),
        race: race.name.clone(),
        player_car: race.story.player_car_type.clone(),
        steps,
        scenes,
    })
}

fn step_of(step: &str, detail: String) -> StepReport {
    StepReport { step: step.to_owned(), detail }
}

/// Who a race's start scene can put on its car channels: the player in the race's forced car, then the race's
/// opponents as `start.rs` plans them (a career race), on grid slots 0, 1, 2...
fn seating(dir: &GameDir, data: &GameplayData, race: &Race) -> Seating {
    let tables = nfsmw_data::car::CarTables::load(dir).ok();
    let resolve_preset = |name: &str| {
        let tables = tables.as_ref()?;
        let ride = tables.preset(name)?;
        let vehicle = nfsmw_data::car::physics::performance::preset::vehicle_name(&tables.attributes, ride);
        Some(format!("{vehicle}:{}", ride.preset_name))
    };
    let player = Seat { name: "player".into(), car: race.story.player_car_type.clone(), stock: false };
    let mut seats = vec![player];
    let options = RaceOptions { career: true, ..RaceOptions::default() };
    let listed = setup::opponents_of(data, race, &options, &resolve_preset);
    let count = listed.len() + 1;
    let pool = super::start::stock_pool(dir);
    for (i, opponent) in listed.into_iter().enumerate() {
        let stock = opponent.car.is_none();
        let car = opponent.car.or_else(|| setup::stock_car(&pool, race.key, i).cloned());
        seats.push(Seat { name: opponent.name, car, stock });
    }
    Seating { race_start: true, grid: (0..seats.len()).collect(), seats, listed: count }
}

#[cfg(test)]
#[path = "scene_report_tests.rs"]
mod tests;
