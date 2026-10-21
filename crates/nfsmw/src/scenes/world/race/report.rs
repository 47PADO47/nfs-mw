//! The post-race report: the racers' laps and places for `PostRace_Results.fng`, and the screen's answer.

use blackbox_race::Race;

use super::{RaceSession, Stage};
use crate::scenes::world::WorldScene;
use nfsmw_data::gameplay::RaceType;

use crate::viewer::{PhotoKind, PhotoReport, RaceAnswer, RaceReport, ReportLap, ReportRacer};

impl WorldScene {
    /// The report of a finished race, from the engine's lap times.
    pub(super) fn race_report(session: &RaceSession) -> Option<RaceReport> {
        let names: Vec<&str> = session.plan.iter().map(|p| p.name.as_str()).collect();
        Some(report_of(session.engine.as_ref()?, &names, session.kind, session.cash_value, session.collisions))
    }

    /// The report, once, for the front end to show.
    pub(in crate::scenes::world) fn take_race_report(&mut self) -> Option<RaceReport> {
        self.race.as_mut()?.report.take()
    }

    /// The post-race screen's answer: run the race again, or end it. `false` when no race waits on the screen.
    pub(in crate::scenes::world) fn answer_race(&mut self, answer: RaceAnswer) -> bool {
        let Some(session) = self.race.take_if(|s| matches!(s.stage, Stage::PostRace { .. })) else { return false };
        let again = (session.name.clone(), session.options);
        // A won career race goes on to the race its `PostRaceActivity` names (the Lua graph's `done` state), which the
        // career then runs before free roam; a loss leaves the chain **[unconfirmed]**.
        let won = session.result.is_some_and(|r| r.place == 1 && r.time.is_some());
        let next = session.chain.clone().filter(|_| won && session.options.career);
        self.end_race(session);
        match (answer, next) {
            (RaceAnswer::Restart, _) => self.queued_race = Some(again),
            (RaceAnswer::Exit, Some(next)) => self.queued_race = Some((next, again.1)),
            (RaceAnswer::Exit, None) => {}
        }
        true
    }
}

/// The report of `engine`: a racer's place at the end of a lap is its rank among the racers that had crossed the same
/// lap's line by then.
fn report_of(engine: &Race, names: &[&str], kind: RaceType, cash: f32, collisions: u32) -> RaceReport {
    let outcomes = engine.outcomes();
    let winner = winner_time(engine);
    let racers = engine.racers();
    let report = racers
        .iter()
        .enumerate()
        .map(|(i, racer)| {
            let mut before = 0.0;
            let laps = racer
                .lap_times
                .iter()
                .enumerate()
                .map(|(k, &at)| {
                    let ahead = racers.iter().filter(|q| q.lap_times.get(k).is_some_and(|&t| t < at)).count();
                    let lap = ReportLap { time: at - before, place: ahead as u32 + 1 };
                    before = at;
                    lap
                })
                .collect();
            let me = outcomes.get(i);
            let time = me.and_then(|o| o.time);
            let ran = racer.finished_at.or(racer.out_at).unwrap_or_else(|| engine.elapsed());
            ReportRacer {
                name: names.get(i).map_or_else(|| format!("Racer {i}"), |n| (*n).to_owned()),
                place: me.map_or(i as u32 + 1, |o| o.place),
                time,
                laps,
                stats: racer.stats,
                average_speed: racer.stats.average_speed(ran),
                time_behind: time_behind(time, winner),
                collisions: if i == 0 { collisions } else { 0 },
                checkpoints: racer.checkpoint_times.clone(),
                knocked_out: racer.out_at.is_some(),
                traps: racer.traps.slots().to_vec(),
            }
        })
        .collect();
    let photo = photo_of(engine, kind, cash);
    RaceReport { racers: report, kind, laps: engine.course().laps.max(1), photo, pursuit: None }
}

/// The finish time of the racer that won, if one finished first.
fn winner_time(engine: &Race) -> Option<f32> {
    engine.outcomes().iter().find(|o| o.place == 1 && o.finished).and_then(|o| o.time)
}

/// Seconds a racer finished behind the winner: none for the winner itself and for a racer with no time.
pub(crate) fn time_behind(time: Option<f32>, winner: Option<f32>) -> Option<f32> {
    let (time, winner) = (time?, winner?);
    let behind = (time - winner).abs();
    if behind <= 0.0 {
        return None;
    }
    Some(behind)
}

/// The win photo: only for a player who finished first. The sprint's speed is the average over the course; a drag
/// race's is the speed at the line; a tollbooth race shows the time left and the bonus time the checkpoints gave.
fn photo_of(engine: &Race, kind: RaceType, cash: f32) -> Option<PhotoReport> {
    let me = engine.racers().first()?;
    let outcome = engine.outcomes().first().copied()?;
    let time = outcome.time.filter(|_| outcome.finished && outcome.place == 1)?;
    let course = engine.course();
    let average = course.lap_length() * course.laps.max(1) as f32 / time.max(0.001);
    let bonus_secs = (1..=me.progress.reached.max(course.checkpoints.len())).map(|n| course.bonus(n)).sum();
    let (kind, speed) = match kind {
        RaceType::Drag => (PhotoKind::Drag, me.speed),
        RaceType::Tollbooth => (PhotoKind::Tollbooth, average),
        _ => (PhotoKind::Sprint, average),
    };
    Some(PhotoReport {
        kind,
        cash: cash.max(0.0).round() as u32,
        time,
        speed,
        time_left: engine.time_left(),
        bonus_secs,
        bounty: 0,
    })
}

#[cfg(test)]
mod tests {
    use blackbox_race::{Course, Gate, RaceConfig};
    use glam::Vec3;

    use super::*;

    fn gate(z: f32) -> Gate {
        Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
    }

    /// Drives racer `index` along z past every gate of a lap and the line.
    fn lap(race: &mut Race, index: usize) {
        race.place_racer(index, Vec3::ZERO);
        for z in [10.0, 150.0, 250.0, 350.0, 400.0] {
            race.move_racer(index, Vec3::new(0.0, 0.0, z));
        }
    }

    #[test]
    fn the_report_has_each_racers_laps_and_the_place_held_at_each() {
        let course = Course::new(Vec3::ZERO, vec![gate(100.0), gate(200.0)], Some(gate(300.0)), 2);
        let mut race = Race::new(course, RaceConfig::default(), &[(true, Vec3::ZERO), (false, Vec3::ZERO)]);
        race.start_now();
        // The opponent is first over the first line (at 20 s), the player second (at 25 s); the player is first over the
        // second line (at 45 s) and finishes first.
        race.tick(20.0);
        lap(&mut race, 1);
        race.tick(5.0);
        lap(&mut race, 0);
        race.tick(20.0);
        lap(&mut race, 0);
        race.tick(5.0);
        lap(&mut race, 1);
        let report = report_of(&race, &["PLAYER", "RIVAL"], RaceType::Circuit, 500.0, 2);
        assert_eq!(report.racers[0].name, "PLAYER");
        let places: Vec<u32> = report.racers[0].laps.iter().map(|l| l.place).collect();
        assert_eq!(places, [2, 1]);
        assert_eq!(report.racers[0].laps[0].time, 25.0);
        assert_eq!(report.racers[0].laps[1].time, 20.0);
        // The race is over when the player finishes: the opponent's second lap is never counted.
        assert_eq!(report.racers[1].laps.iter().map(|l| l.place).collect::<Vec<_>>(), [1]);
        assert_eq!((report.racers[0].place, report.racers[1].place), (1, 2));
        assert_eq!(report.racers[0].time, Some(45.0));
        assert_eq!((report.racers[0].collisions, report.racers[1].collisions), (2, 0));
        assert_eq!(report.racers[0].time_behind, None, "the winner is behind nobody");
        assert_eq!(report.racers[1].time_behind, None, "a racer without a time has none");
        assert_eq!((report.kind, report.laps), (RaceType::Circuit, 2));
        let photo = report.photo.expect("the player won");
        assert_eq!((photo.kind, photo.cash, photo.time), (PhotoKind::Sprint, 500, 45.0));
        assert!(photo.speed > 0.0);
    }

    #[test]
    fn the_time_behind_is_the_gap_to_the_winner() {
        assert_eq!(time_behind(Some(175.2), Some(172.5)), Some(175.2 - 172.5));
        assert_eq!(time_behind(Some(172.5), Some(172.5)), None);
        assert_eq!(time_behind(None, Some(172.5)), None);
        assert_eq!(time_behind(Some(10.0), None), None);
    }

    #[test]
    fn a_racer_without_a_name_gets_one() {
        let course = Course::new(Vec3::ZERO, vec![gate(100.0)], Some(gate(300.0)), 1);
        let race = Race::new(course, RaceConfig::default(), &[(true, Vec3::ZERO), (false, Vec3::ZERO)]);
        let report = report_of(&race, &["PLAYER"], RaceType::Sprint, 0.0, 0);
        assert_eq!(report.racers[1].name, "Racer 1");
        assert!(report.racers[0].laps.is_empty());
        assert_eq!(report.photo, None, "no photo for a race nobody finished");
    }
}
