//! The loading stage: waiting for the player's car to stand on the grid and for the racers' models, then putting the
//! racers on the grid and starting the countdown.

use blackbox_race::{CatchUp, Race};
use glam::Vec3;
use nfsmw_data::pursuit::ai_vehicle;

use super::{RaceSession, Stage};
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
        if !self.drive.as_ref().is_some_and(|d| d.has_sim() && d.waiting_for_road().is_none()) {
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
        let Some(drive) = self.drive.as_mut() else { return };
        let player_physics = Vec3::from(space::to_physics(session.plan[0].spawn.position));
        let player_perf = drive.performance();
        let speed = if prep.rolling { prep.initial_speed } else { 0.0 };
        let mut kept = vec![session.plan[0].clone()];
        let mut grid = vec![(true, player_physics)];
        if let Some(traffic) = self.traffic.as_mut() {
            let collision = self.residency.collision();
            for plan in session.plan.iter().skip(1) {
                let Some(model) = plan.model.clone() else { continue };
                let spec = RacerSpawn {
                    id: kept.len(),
                    ai: ai_vehicle(self.physics.database(), &model),
                    model,
                    spawn: plan.spawn,
                    player: player_perf,
                    bias: plan.bias,
                    base_skill: plan.skill,
                    speed,
                    held: !prep.rolling,
                    person: plan.person,
                };
                if !traffic.spawn_racer(&spec, collision, &self.physics.surfaces) {
                    log::warn!("race: {} could not be put on the grid", plan.name);
                    continue;
                }
                grid.push((plan.person.is_some(), Vec3::from(space::to_physics(plan.spawn.position))));
                kept.push(plan.clone());
            }
        }
        session.plan = kept;
        session.emu = vec![None; grid.len()];
        session.glue = (0..grid.len()).map(|_| CatchUp::new(prep.gains)).collect();
        let mut engine = Race::new(prep.course, prep.config, &grid);
        let started = match prep.rolling {
            true => engine.start_now(),
            false => engine.begin_countdown(),
        };
        session.engine = Some(engine);
        session.stage = Stage::Running;
        self.race_events(session, started);
    }
}
