//! The end of a race: the player's result, the career glue and the clean-up.

use super::{Banner, RaceResult, RaceSession, Stage};
use crate::scenes::world::WorldScene;

/// Seconds the result card stays before the world goes back to free roam.
const RESULT_SECS: f32 = 10.0;

impl WorldScene {
    /// The engine says the race is over: the player's place is fixed, the career hears of it and the card goes up.
    pub(super) fn finish_race(&mut self, session: &mut RaceSession) {
        let Some(engine) = session.engine.as_ref() else { return };
        let outcomes = engine.outcomes();
        let racers = outcomes.len();
        let me = outcomes[0];
        // A player who did not finish (out, or the time ran out) counts as last.
        let place = if me.finished { me.place } else { racers as u32 };
        session.result = Some(RaceResult { place, racers, time: me.time });
        session.stage = Stage::Results { left: RESULT_SECS };
        session.banner = Banner::default();
        log::info!(
            "race: {} over: place {place}/{racers}{}",
            session.name,
            me.time.map_or_else(String::new, |t| format!(", {t:.1} s"))
        );
        if let Some(drive) = self.drive.as_mut() {
            drive.hold_on_grid(false);
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.release_racers();
            for id in 1..racers {
                if let Some(state) = traffic.racer_mut(id) {
                    state.set_goal(None);
                }
            }
        }
        if session.options.career {
            let place = u8::try_from(place).unwrap_or(u8::MAX);
            self.career_race_finished(&session.event, place, session.cash_value);
        }
    }

    /// Takes the race down: the racers leave, the traffic count is given back and the car is let go.
    pub(super) fn end_race(&mut self, session: RaceSession) {
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.clear_racers();
        }
        self.traffic_manual = session.previous_traffic;
        if let Some(drive) = self.drive.as_mut() {
            drive.hold_on_grid(false);
        }
        log::info!("race: {} ended", session.name);
        if session.options.career {
            self.career_race_left();
        }
    }

    /// Ends the race now, with no result (the console's `race stop`, or the world going away).
    pub(in crate::scenes::world) fn abandon_race(&mut self) -> bool {
        let Some(session) = self.race.take() else { return false };
        self.end_race(session);
        true
    }
}
