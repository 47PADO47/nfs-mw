//! The running stage: the clock, the racers' positions, the checkpoints, the goals of the opponents and the catch-up.

use blackbox_race::{Phase, RaceEvent};
use glam::Vec3;

use super::{Banner, RaceSession, Stage, setup};
use crate::input::{Action, ActionState};
use crate::scenes::world::drive::STEP;
use crate::scenes::world::{WorldScene, space};

/// Seconds the world waits for a front end to show the post-race screen before it ends the race (a run without one).
const POST_RACE_WAIT: f32 = 10.0;
/// Seconds the banner of a checkpoint or lap stays.
const BANNER_SECS: f32 = 1.5;
/// Seconds the GO stays: the length of the HUD's `GO` script (3200 ms, `docs/provenance/race-mode.md` section 9).
const GO_SECS: f32 = 3.0;

impl WorldScene {
    /// One frame of the race: loading, a challenge or the result card, on the real frame time `frame_dt` (the result
    /// card and the loading wait use real time). A running race steps in [`WorldScene::race_tick`] instead.
    pub(in crate::scenes::world) fn update_race(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        input: &ActionState,
        frame_dt: f32,
    ) {
        self.update_race_mods();
        self.start_queued_race();
        let Some(mut session) = self.race.take() else { return };
        let keep = match session.stage {
            Stage::Loading { .. } => self.race_loading(&mut session, frame_dt),
            Stage::Intro { .. } => self.race_intro(&mut session, frame_dt),
            // `frame_dt` is 0 while the world is frozen (a scene or a story movie plays), so the challenge's clock stops.
            Stage::Running if session.challenge.is_some() => self.challenge_running(renderer, &mut session, frame_dt),
            Stage::Running => true,
            Stage::Results { .. } => self.race_results(&mut session, input, frame_dt),
            Stage::PostRace { .. } => self.race_post(&mut session, frame_dt),
            Stage::Outro { .. } => self.race_outro(&mut session, frame_dt),
        };
        session.banner.left = (session.banner.left - frame_dt).max(0.0);
        match keep {
            true => self.race = Some(session),
            false => self.end_race(session),
        }
    }

    /// One physics tick of a running race: the clock, the racers' positions and checkpoints, the goals and the catch-up.
    /// It runs inside the tick loop, after the physics and the police, so a crossing is tested at the tick it happens
    /// and the race does not depend on how ticks group into frames (`docs/specs/multiplayer/determinism.md`, D2).
    pub(in crate::scenes::world) fn race_tick(&mut self) {
        let Some(mut session) = self.race.take() else { return };
        let keep = match session.stage {
            Stage::Running if session.challenge.is_none() => self.race_running(&mut session, STEP),
            _ => true,
        };
        match keep {
            true => self.race = Some(session),
            false => self.end_race(session),
        }
    }

    /// Starts the race a quick race asked for, as soon as the car exists and no race is running.
    fn start_queued_race(&mut self) {
        if self.race.is_some() || self.drive.is_none() {
            return;
        }
        let Some((query, options)) = self.queued_race.take() else { return };
        match self.start_race(&query, options) {
            Ok(line) => log::info!("race: {line}"),
            Err(why) => log::warn!("race {query}: {why}"),
        }
    }

    /// The player's place in physics space, once the car stands.
    fn player_physics(&self) -> Option<Vec3> {
        let drive = self.drive.as_ref().filter(|d| d.has_sim() && d.waiting_for_road().is_none())?;
        Some(Vec3::from(space::to_physics(drive.position())))
    }

    fn race_running(&mut self, session: &mut RaceSession, dt: f32) -> bool {
        let Some(engine) = session.engine.as_mut() else { return false };
        let mut events = engine.tick(dt);
        let player = self.player_physics();
        if let Some(position) = player {
            engine.set_speed(0, self.drive.as_ref().map_or(0.0, |d| d.speed().abs()));
            events.extend(engine.move_racer(0, position));
        }
        events.extend(self.step_racers(session, player, dt));
        self.count_collisions(session);
        self.drag_assist(session);
        self.demo_step(session, dt);
        self.race_goals(session);
        self.race_glue(session, dt);
        self.race_log(session, dt);
        self.race_events(session, events);
        true
    }

    /// Adds the player's collisions with traffic since the last frame to the race's count, while the player is racing.
    fn count_collisions(&mut self, session: &mut RaceSession) {
        let Some(traffic) = self.traffic.as_mut() else { return };
        let hits = traffic.take_traffic_collisions();
        let racing = session.engine.as_ref().is_some_and(|e| e.phase() == Phase::Racing && e.racers()[0].racing());
        if racing {
            session.collisions += hits;
        }
    }

    /// A line per racer every ten seconds at the debug level: where it is, how fast, and how far from its next gate.
    fn race_log(&self, session: &mut RaceSession, dt: f32) {
        session.log_clock += dt;
        if session.log_clock < 10.0 || !log::log_enabled!(log::Level::Debug) {
            return;
        }
        session.log_clock = 0.0;
        let (Some(engine), Some(traffic)) = (session.engine.as_ref(), self.traffic.as_ref()) else { return };
        for (id, position, speed) in traffic.racers() {
            let position = session.emu.get(id).copied().flatten().unwrap_or(position);
            let racer = &engine.racers()[id];
            let gate = next_gate(engine.course(), racer.progress.reached);
            let away = gate.map_or(-1.0, |g| g.distance(position));
            let at = gate.unwrap_or_default();
            log::debug!(
                "race: racer {id} at ({:.0}, {:.0}, {:.0}) {speed:.0} m/s, {:.0}% done, {away:.0} m from its next gate at ({:.0}, {:.0}, {:.0}), {} checkpoint(s)",
                position.x,
                position.y,
                position.z,
                racer.percent,
                at.x,
                at.y,
                at.z,
                racer.progress.reached
            );
        }
    }

    /// Each opponent drives to the next gate of its own progress; a racer that is done has no goal and cruises on.
    fn race_goals(&mut self, session: &RaceSession) {
        let (Some(engine), Some(traffic)) = (session.engine.as_ref(), self.traffic.as_mut()) else { return };
        for (id, racer) in engine.racers().iter().enumerate().skip(1) {
            let goal = match racer.done() {
                true => None,
                false => next_aim(engine.course(), racer.progress.reached),
            };
            if let Some(state) = traffic.racer_mut(id) {
                state.set_goal(goal);
            }
        }
    }

    /// The catch-up, once a second while the race runs and the player has not finished.
    fn race_glue(&mut self, session: &mut RaceSession, dt: f32) {
        if !session.catch_up {
            return;
        }
        session.glue_clock += dt;
        if session.glue_clock < 1.0 {
            return;
        }
        session.glue_clock = 0.0;
        let (Some(engine), Some(traffic)) = (session.engine.as_ref(), self.traffic.as_mut()) else { return };
        let racers = engine.racers();
        if engine.phase() != Phase::Racing || racers[0].done() {
            return;
        }
        let length = engine.course().length();
        let player = racers[0].percent;
        let half = session.kind == nfsmw_data::gameplay::RaceType::SpeedTrap;
        for (id, racer) in racers.iter().enumerate().skip(1).filter(|(_, r)| !r.done()) {
            let error = blackbox_race::CatchUp::error(length, player, racer.percent, 1.0);
            let skill = session.glue[id].update(error, 1.0);
            let Some(state) = traffic.racer_mut(id) else { continue };
            state.glue = setup::scale_glue(skill, state.base_skill, half);
        }
    }

    /// Carries out what the engine raised, in order.
    pub(super) fn race_events(&mut self, session: &mut RaceSession, events: Vec<RaceEvent>) {
        let total_checkpoints = session.engine.as_ref().map_or(0, |e| e.course().checkpoints.len());
        let laps = session.engine.as_ref().map_or(1, |e| e.course().laps);
        for event in events {
            log::debug!("race: {event:?}");
            match event {
                RaceEvent::Count(n) => flash(&mut session.banner, n.to_string(), 1.0),
                RaceEvent::Go => {
                    flash(&mut session.banner, "GO".into(), GO_SECS);
                    self.release_grid();
                }
                RaceEvent::TimeUp => flash(&mut session.banner, "Time is up".into(), BANNER_SECS * 2.0),
                RaceEvent::Checkpoint { racer: 0, number } => {
                    flash(&mut session.banner, format!("Checkpoint {number}/{total_checkpoints}"), BANNER_SECS)
                }
                RaceEvent::Lap { racer: 0, lap } => {
                    if let Some(engine) = session.engine.as_ref() {
                        let now = engine.elapsed();
                        crate::frontend::custom_race::records::note_lap(&session.event, now - session.lap_mark);
                        session.lap_mark = now;
                    }
                    flash(&mut session.banner, format!("Lap {}/{laps}", lap + 1), BANNER_SECS)
                }
                RaceEvent::Finished { racer: 0, .. } => {
                    flash(&mut session.banner, "Finished".into(), BANNER_SECS * 2.0)
                }
                RaceEvent::KnockedOut { racer: 0 } => {
                    flash(&mut session.banner, "Knocked out".into(), BANNER_SECS * 2.0)
                }
                RaceEvent::Bonus { racer: 0, seconds } => {
                    flash(&mut session.banner, format!("+{seconds:.0} s"), BANNER_SECS)
                }
                RaceEvent::SpeedTrap { racer: 0, kmh, .. } => {
                    crate::frontend::custom_race::records::note_speed(&session.event, kmh);
                    flash(&mut session.banner, format!("+{kmh:.0} km/h"), BANNER_SECS)
                }
                RaceEvent::KnockedOut { racer } => {
                    if let Some(traffic) = self.traffic.as_mut() {
                        traffic.remove_racer(racer);
                    }
                }
                RaceEvent::Over => self.finish_race(session),
                RaceEvent::Finished { racer, time } => log::info!("race: racer {racer} finished in {time:.1} s"),
                RaceEvent::Checkpoint { .. }
                | RaceEvent::Lap { .. }
                | RaceEvent::Bonus { .. }
                | RaceEvent::SpeedTrap { .. } => {}
            }
        }
    }

    /// GO: the player's car and the racers' cars are let go.
    fn release_grid(&mut self) {
        if let Some(drive) = self.drive.as_mut() {
            drive.hold_on_grid(false);
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.release_racers();
        }
    }

    /// The race-over message stays for its duration, or until Interact; then the post-race screen has the race.
    fn race_results(&mut self, session: &mut RaceSession, input: &ActionState, dt: f32) -> bool {
        let Stage::Results { left } = session.stage else { return true };
        let left = left - dt;
        session.stage = Stage::Results { left };
        if left > 0.0 && !input.just_pressed(Action::Interact) {
            return true;
        }
        // A challenge has no lap table: its report is the chase's numbers (`PostRace_Pursuit.fng`).
        session.report = match &session.challenge {
            Some(challenge) => challenge.last.map(|pursuit| crate::viewer::RaceReport {
                racers: Vec::new(),
                kind: nfsmw_data::gameplay::RaceType::Sprint,
                laps: 1,
                photo: None,
                pursuit: Some(pursuit),
            }),
            None => Self::race_report(session),
        };
        let Some(report) = session.report.as_ref() else { return false };
        // The win photo is the world seen from the replay camera the race names, through the photo screen's window.
        if report.photo.is_some() {
            self.start_photo_shot(session.origin.race.params.finish_camera.as_deref(), session.kind);
        }
        session.stage = Stage::PostRace { left: POST_RACE_WAIT };
        true
    }

    /// The post-race screen is up (the world is frozen then, so this does not run), or no front end takes the report:
    /// the race ends once the wait is over.
    fn race_post(&mut self, session: &mut RaceSession, dt: f32) -> bool {
        let Stage::PostRace { left } = session.stage else { return true };
        let left = left - dt;
        session.stage = Stage::PostRace { left };
        left > 0.0
    }
}

fn flash(banner: &mut Banner, text: String, secs: f32) {
    *banner = Banner { text, left: secs, serial: banner.serial.wrapping_add(1) };
}

/// How far past a gate's centre an opponent aims, metres. A path that ends at the gate would stop the car short of it
/// (a path's end is a dead end), so the racer aims beyond it and crosses it on the way **[unconfirmed]**.
const OVERSHOOT: f32 = 40.0;

/// Where an opponent drives to for the next gate: a point `OVERSHOOT` beyond its centre, along its heading.
pub(super) fn next_aim(course: &blackbox_race::Course, reached: usize) -> Option<Vec3> {
    let gate = course.checkpoints.get(reached).or(course.finish.as_ref())?;
    Some(gate.centre + gate.forward * OVERSHOOT)
}

/// The centre of the next gate in physics space: checkpoint `reached` (0-based), else the finish line.
pub(super) fn next_gate(course: &blackbox_race::Course, reached: usize) -> Option<Vec3> {
    course.checkpoints.get(reached).map(|g| g.centre).or_else(|| course.finish.as_ref().map(|g| g.centre))
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_race::{Course, Gate};

    fn gate(z: f32) -> Gate {
        Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
    }

    #[test]
    fn the_next_gate_is_the_next_checkpoint_then_the_finish() {
        let course = Course::new(Vec3::ZERO, vec![gate(10.0), gate(20.0)], Some(gate(30.0)), 1);
        assert_eq!(next_gate(&course, 0).unwrap().z, 10.0);
        assert_eq!(next_gate(&course, 1).unwrap().z, 20.0);
        assert_eq!(next_gate(&course, 2).unwrap().z, 30.0);
        let open = Course { finish: None, ..course };
        assert_eq!(next_gate(&open, 2), None);
    }

    #[test]
    fn an_opponent_aims_past_the_gate_along_its_heading() {
        let course = Course::new(Vec3::ZERO, vec![gate(10.0)], Some(gate(30.0)), 1);
        assert_eq!(next_aim(&course, 0).unwrap(), Vec3::new(0.0, 0.0, 10.0 + OVERSHOOT));
        assert_eq!(next_aim(&course, 1).unwrap().z, 30.0 + OVERSHOOT);
        assert_eq!(next_aim(&course, 2).unwrap().z, 30.0 + OVERSHOOT, "past the last checkpoint it aims at the finish");
    }

    #[test]
    fn a_flash_replaces_the_banner() {
        let mut banner = Banner::default();
        flash(&mut banner, "GO".into(), 1.5);
        assert_eq!((banner.text.as_str(), banner.left), ("GO", 1.5));
    }
}
