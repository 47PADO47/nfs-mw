//! The loading stage: waiting for the player's car to stand on the grid and for the racers' models, then putting the
//! racers on the grid and starting the countdown.

use blackbox_race::Race;
use glam::Vec3;
use nfsmw_data::car::physics::{CarPhysics, PerformanceCatalog, PhysicsData};
use nfsmw_data::pursuit::ai_vehicle;

use super::{RaceSession, Stage, intro, scripted_chase::CHASE_EVENT, shortcuts};
use crate::scenes::world::ai::{ModelState, RacerSpawn};
use crate::scenes::world::{WorldScene, space};

/// Seconds the models may take before the racers that have not loaded are left out.
const MODEL_PATIENCE: f32 = 45.0;
/// Stock cars tried for an opponent whose car failed to load.
const FALLBACKS: usize = 3;

impl WorldScene {
    /// One frame of the loading stage. Returns false when the race cannot go on (it is dropped).
    pub(super) fn race_loading(&mut self, session: &mut RaceSession, dt: f32) -> bool {
        let Stage::Loading { waited } = session.stage else { return true };
        session.stage = Stage::Loading { waited: waited + dt };
        if !self.players.lead().is_some_and(|d| d.has_sim() && d.waiting_for_road().is_none()) {
            return true;
        }
        if !self.residency.complete() {
            return true;
        }
        if !self.racer_models_ready(session, waited > MODEL_PATIENCE) {
            return true;
        }
        self.put_racers_on_the_grid(session);
        true
    }

    /// Asks for each racer's model; swaps in a stock car for one that failed. True when every model is loaded (or, once
    /// `give_up`, when the ones that are not have been left out).
    fn racer_models_ready(&mut self, session: &mut RaceSession, give_up: bool) -> bool {
        let Some(traffic) = self.traffic.as_mut() else {
            session.plan.truncate(1);
            return true;
        };
        let mut ready = true;
        let mut dropped = Vec::new();
        for (i, plan) in session.plan.iter_mut().enumerate().skip(1) {
            let Some(model) = plan.model.clone() else { continue };
            match traffic.racer_model_state(&model) {
                ModelState::Loaded => {}
                ModelState::Pending => {
                    traffic.request_racer_model(&model);
                    ready = false;
                    if give_up {
                        dropped.push(i);
                    }
                }
                ModelState::Failed => {
                    plan.tried.push(model);
                    let next = session.pool.iter().find(|c| !plan.tried.contains(c)).cloned();
                    match (next, plan.tried.len() <= FALLBACKS) {
                        (Some(next), true) => {
                            log::warn!(
                                "race: {} could not load; {} takes {next}",
                                plan.tried.last().unwrap(),
                                plan.name
                            );
                            traffic.request_racer_model(&next);
                            plan.model = Some(next);
                            ready = false;
                        }
                        _ => dropped.push(i),
                    }
                }
            }
        }
        for i in dropped.into_iter().rev() {
            log::warn!("race: {} is left out: its car did not load", session.plan[i].name);
            // The people sit on the grid right after the lead: racer `i` is `race_people[i - 1]`.
            if session.plan[i].person.is_some() && i - 1 < self.race_people.len() {
                self.race_people.remove(i - 1);
            }
            session.plan.remove(i);
            ready = session
                .plan
                .iter()
                .skip(1)
                .all(|p| p.model.as_deref().is_none_or(|m| traffic.racer_model_state(m) == ModelState::Loaded));
        }
        ready
    }

    /// Everything is loaded: the racers are spawned, the engine is made and the countdown (or the rolling start) begins.
    fn put_racers_on_the_grid(&mut self, session: &mut RaceSession) {
        let Some(prep) = session.setup.take() else { return };
        session.catch_up = prep.catch_up;
        let Some(drive) = self.players.lead_mut() else { return };
        let player_physics = Vec3::from(space::to_physics(session.plan[0].spawn.position));
        let player_perf = drive.ai_performance();
        let player_nitrous = drive.nitrous_hardware();
        if !session.options.career {
            session.player_top_speed_rating = nfsmw_data::car::physics::ratings::RatingBook::load(&self.dir)
                .ok()
                .and_then(|book| book.for_car(&drive.car_name))
                .map_or(1.0, |endpoints| endpoints.rating(drive.performance)[0]);
        }
        let speed = if prep.rolling { prep.initial_speed } else { 0.0 };
        let mut kept = vec![session.plan[0].clone()];
        let mut grid = vec![(!self.race_lead_idle, player_physics)];
        if let Some(traffic) = self.traffic.as_mut() {
            let collision = self.residency.collision();
            // Searched at the start with the walls up (`start.rs`): an intro scene may have put them down since.
            let shortcuts = std::mem::take(&mut session.shortcuts);
            // The people of the grid are the first racers after the lead; one that cannot stand there is dropped from
            // `race_people` so the next person keeps their racer number.
            let mut seated_people = 0;
            for plan in session.plan.iter().skip(1) {
                let Some(model) = plan.model.clone() else { continue };
                let car_type = model.split(':').next().unwrap_or(&model);
                let physics = matched_racer_physics(
                    &self.physics,
                    session.options.career,
                    &session.event,
                    &model,
                    plan.person.is_some(),
                )
                .and_then(|result| match result {
                    Ok(physics) => Some(physics),
                    Err(error) => {
                        log::warn!("race: {model} keeps original equipment: {error:#}");
                        None
                    }
                });
                let spec = RacerSpawn {
                    id: kept.len(),
                    ai: ai_vehicle(self.physics.database(), car_type),
                    physics,
                    model,
                    spawn: plan.spawn,
                    player: player_perf,
                    player_nitrous,
                    bias: plan.bias,
                    base_skill: plan.skill,
                    speed: if plan.rolling_speed.is_some() { 0.0 } else { speed },
                    held: !prep.rolling || plan.rolling_speed.is_some(),
                    person: plan.person,
                };
                if !traffic.spawn_racer(&spec, collision, &self.physics.surfaces) {
                    log::warn!("race: {} could not be put on the grid", plan.name);
                    if plan.person.is_some() && seated_people < self.race_people.len() {
                        self.race_people.remove(seated_people);
                    }
                    continue;
                }
                if plan.person.is_some() {
                    seated_people += 1;
                }
                if let Some(state) = traffic.racer_mut(spec.id) {
                    state.drag = session.kind == nfsmw_data::gameplay::RaceType::Drag;
                }
                if let Some(base) = &session.route {
                    let id = kept.len();
                    let taken =
                        shortcuts::rolls(&shortcuts, plan.skill, shortcuts::seed_of(session.origin.race.key, id));
                    traffic.set_racer_route(id, shortcuts::racer_route(traffic, base, &shortcuts, &taken));
                }
                grid.push((plan.person.is_some(), Vec3::from(space::to_physics(plan.spawn.position))));
                kept.push(plan.clone());
            }
            session.shortcuts = shortcuts;
        }
        session.plan = kept;
        let difficulty = nfsmw_data::gameplay::RaceParams {
            difficulty: session.options.difficulty.unwrap_or(session.origin.race.params.difficulty),
            ..Default::default()
        }
        .difficulty_class();
        let phases = self
            .traffic
            .as_mut()
            .map_or_else(|| vec![0.0; grid.len()], |traffic| traffic.race_catch_up_phases(grid.len()));
        // The career's adaptive bonus is the profile's stored value; no retail path was found that raises it from zero
        // (`ai-racers-catchup.md` section 4).
        let context = match session.options.career {
            true => super::catch_up::Context::Career { adaptive: 0.0, boss: session.origin.race.story.boss_race },
            false => super::catch_up::Context::Quick { difficulty },
        };
        session.glue = phases.into_iter().map(|phase| super::catch_up::Glue::new(context).with_phase(phase)).collect();
        let lead = match prep.rolling {
            true => 0,
            false => (prep.config.countdown * 60.0).ceil().max(0.0) as u32,
        };
        session.engine = Some(Race::new(prep.course, prep.config, &grid));
        self.trace_race_begins(session);
        // A session's race waits on the grid for the server to release the countdown (`hold.rs`).
        if std::mem::take(&mut self.race_hold) {
            session.stage = Stage::Staged { lead };
            return;
        }
        // The intro plays on the grid before the countdown (`intro.rs`); a race with no steps starts now.
        let steps = match self.rules.race_screens {
            true => intro::intro_queue(session),
            false => Default::default(),
        };
        if steps.is_empty() {
            let nis_shell = session.origin.race.story.nis_shell;
            self.after_intro(session, nis_shell);
            return;
        }
        session.stage = Stage::Intro { steps, current: intro::Playing::Idle };
    }
}

/// Native race-vehicle matching gives quick opponents maximum ordinary hardware. The same
/// request is verified on Razor's scripted second race; his named preset still supplies the looks.
fn matched_racer_physics(
    data: &PhysicsData,
    career: bool,
    event: &str,
    model: &str,
    human: bool,
) -> Option<anyhow::Result<CarPhysics>> {
    if human || (career && !(event == CHASE_EVENT && model.eq_ignore_ascii_case("mustanggt:razormustang"))) {
        return None;
    }
    let car_type = model.split(':').next().unwrap_or(model);
    Some(
        PerformanceCatalog::read(data.database(), car_type)
            .and_then(|catalog| data.car_with_performance(car_type, catalog.maximum())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs installed MustangGT equipment records"]
    fn installed_second_razor_uses_native_matched_hardware() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let data = PhysicsData::load(&dir).unwrap();
        let model = "mustanggt:RAZORMUSTANG";
        let stock = data.car("mustanggt").unwrap().spec;
        let matched = matched_racer_physics(&data, true, CHASE_EVENT, model, false).unwrap().unwrap().spec;

        // Native live engine produced 646.04 hp: its upper ordinary torque endpoint multiplied
        // by its high-RPM induction boost. The transmission had six forward gears.
        let engine_hp = matched.engine.torque.last().unwrap() * matched.engine.max_rpm / 5252.0;
        let max_hp = engine_hp * (1.0 + matched.induction.high_boost);
        assert!((max_hp - 646.04).abs() < 0.1, "native peak horsepower: {max_hp}");
        assert_eq!(matched.transmission.gear_ratio.len(), 8);
        assert_eq!(stock.transmission.gear_ratio.len(), 7);
        assert_eq!(stock.induction.high_boost, 0.0);
        assert!(matched_racer_physics(&data, true, "16.1.0", model, false).is_none());
        assert!(matched_racer_physics(&data, true, CHASE_EVENT, model, true).is_none());
        assert!(matched_racer_physics(&data, true, CHASE_EVENT, "gto", false).is_none());
    }
}
