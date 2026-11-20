//! What the race trace reads of the running race (`unattended.rs` writes it): the header at the green light and each
//! racer's row of a `sample` line. Every value comes from the race and the cars as they run, not from the command line.
//! The layout is `parity/race_trace`; `docs/testing.md` ("Race trace") lists the keys.

use nfsmw_data::gameplay::RaceParams;

use super::RaceSession;
use crate::parity::race_trace::{
    Build, Controls, Difficulty, DriverRow, Header, Mode, RacerRow, SCHEMA, SampleRow, Traffic,
};
use crate::scenes::world::ai::RaceDriving;
use crate::scenes::world::{WorldScene, space};

/// The quick-race difficulty `session` runs with, as `load.rs` takes it for the catch-up and `setup.rs` for the skills:
/// the option, else the race's `DifficultyLevel`. `None` in a career race, which uses neither.
pub(super) fn quick_difficulty(session: &RaceSession) -> Option<Difficulty> {
    if session.options.career {
        return None;
    }
    let level = session.options.difficulty.unwrap_or(session.origin.race.params.difficulty);
    let class = RaceParams { difficulty: level, ..Default::default() }.difficulty_class();
    Some(Difficulty::new(level, class))
}

/// A sample row of an AI-driven opponent from what its driver and car did.
fn opponent_row(racer: usize, position: [f32; 3], speed: f32, driving: Option<RaceDriving>) -> SampleRow {
    let controls = driving.map(|d| Controls {
        throttle: d.throttle,
        brake: d.brake,
        steering: d.steering,
        gear: d.gear,
        rpm: d.rpm,
    });
    let driver = driving.map(|d| DriverRow {
        target_speed: d.target_speed,
        base_skill: d.base_skill,
        catch_up: d.catch_up,
        skill: d.skill,
        catch_up_force: d.catch_up_force,
        stand_in: d.stand_in,
    });
    SampleRow { racer, position, speed, controls, driver }
}

impl WorldScene {
    /// Every racer on the road as a sample row, in racer order: the player's car first, then the opponents.
    pub(super) fn race_trace_rows(&self) -> Vec<SampleRow> {
        let mut rows = Vec::new();
        if let Some((drive, sim)) = self.players.lead().and_then(|drive| Some((drive, drive.sim()?))) {
            let telemetry = sim.telemetry();
            // The autopilot's driver, while it has the car (`autopilot.rs`): no catch-up reaches the player's car.
            let driver = self.unattended.pilot.as_ref().filter(|_| self.unattended.engaged).map(|pilot| {
                let (target_speed, base_skill, catch_up, skill) = pilot.trace_state();
                DriverRow { target_speed, base_skill, catch_up, skill, catch_up_force: 0.0, stand_in: false }
            });
            rows.push(SampleRow {
                racer: 0,
                position: drive.position().to_array(),
                speed: telemetry.speed_mps.abs(),
                controls: Some(Controls {
                    throttle: sim.gas_pedal(),
                    brake: sim.brake_pedal(),
                    steering: sim.steering_input(),
                    gear: telemetry.gear,
                    rpm: telemetry.rpm,
                }),
                driver,
            });
        }
        let Some(traffic) = self.traffic.as_ref() else { return rows };
        let driving: Vec<(usize, RaceDriving)> = traffic.racers_driving().collect();
        let first = rows.len();
        rows.extend(traffic.racers().map(|(racer, at, speed)| {
            let mine = driving.iter().find(|(id, _)| *id == racer).map(|(_, d)| *d);
            opponent_row(racer, space::to_render(at.to_array()).to_array(), speed, mine)
        }));
        rows[first..].sort_by_key(|row| row.racer);
        rows
    }

    /// The race's header as the race stands now (the green light): its rules, and each racer's skill and grid place.
    pub(super) fn race_trace_header(&self, session: &RaceSession) -> Option<Header> {
        let engine = session.engine.as_ref()?;
        let rows = self.race_trace_rows();
        let player_car = self.players.lead().map(|drive| drive.car_name.clone());
        let autopilot = self.unattended.plan.autopilot;
        let racers = session.plan.iter().zip(engine.racers()).enumerate().map(|(racer, (plan, state))| {
            let row = rows.iter().find(|row| row.racer == racer);
            let skill = match (racer, state.human) {
                (0, _) => autopilot.map(|pilot| pilot.skill),
                (_, true) => None,
                _ => row.and_then(|row| row.driver).map(|driver| driver.base_skill),
            };
            RacerRow {
                racer,
                name: plan.name.clone(),
                car: plan.model.clone().or_else(|| player_car.clone().filter(|_| racer == 0)),
                human: state.human,
                skill,
                grid: row.map(|row| row.position),
            }
        });
        let course = engine.course();
        let (level, pattern) = session.traffic_context();
        Some(Header {
            schema: SCHEMA,
            created_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_millis() as u64),
            run: self.run_header(),
            build: Build::current(),
            event: session.event.clone(),
            race: session.name.clone(),
            race_type: format!("{:?}", session.kind).to_ascii_lowercase(),
            mode: Mode::of(session.options.career),
            difficulty: quick_difficulty(session),
            catch_up: session.catch_up,
            traffic: Traffic { level, pattern: pattern.map(str::to_owned), cars: self.traffic_wanted() },
            cops: !self.extra.testing.police_off && !self.bars_cops_in(session),
            opponents: engine.racers().len().saturating_sub(1),
            laps: course.laps,
            checkpoints_per_lap: course.checkpoints.len(),
            finish_line: course.finish.is_some(),
            rolling_start: session.origin.race.params.rolling_start,
            racers: racers.collect(),
            autopilot: autopilot.map(Into::into),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn driving(stand_in: bool) -> RaceDriving {
        RaceDriving {
            throttle: 0.75,
            brake: 0.0,
            steering: -0.5,
            gear: 3,
            rpm: 6500.0,
            target_speed: 52.0,
            base_skill: 0.375,
            catch_up: 0.25,
            skill: 0.625,
            catch_up_force: 0.0,
            stand_in,
        }
    }

    #[test]
    fn an_opponents_row_carries_its_controls_and_what_its_driver_asked_for() {
        let row = opponent_row(2, [1.0, 2.0, 3.0], 40.0, Some(driving(false)));
        let line = serde_json::to_value(row).unwrap();
        assert_eq!(
            line,
            serde_json::json!({
                "racer": 2, "position": [1.0, 2.0, 3.0], "speed": 40.0,
                "controls": {"throttle": 0.75, "brake": 0.0, "steering": -0.5, "gear": 3, "rpm": 6500.0},
                "driver": {"target_speed": 52.0, "base_skill": 0.375, "catch_up": 0.25, "skill": 0.625,
                           "catch_up_force": 0.0, "stand_in": false},
            })
        );
        let emulated = serde_json::to_value(opponent_row(2, [0.0; 3], 1.0, Some(driving(true)))).unwrap();
        assert_eq!(emulated["driver"]["stand_in"], serde_json::json!(true));
        let unknown = serde_json::to_value(opponent_row(5, [0.0; 3], 0.0, None)).unwrap();
        assert!(unknown.get("controls").is_none() && unknown.get("driver").is_none(), "no car to read: no keys");
    }
}
