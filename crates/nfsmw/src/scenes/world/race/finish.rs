//! The end of a race: the player's result, the career glue and the clean-up.

use blackbox_race::Outcome;

use super::coop_finish::coop_place;
use super::{Banner, RaceResult, RaceSession, Stage};
use crate::scenes::world::WorldScene;

/// Seconds the race-over message stays: `RaceOverMessageDuration` of a race is 3 s (`docs/specs/career-scripts-events.md`
/// section 1.1, read from the career's Lua graph).
pub(super) const RACE_OVER_SECS: f32 = 3.0;

impl WorldScene {
    /// The engine says the race is over: the player's place is fixed, the career hears of it and the card goes up.
    pub(super) fn finish_race(&mut self, session: &mut RaceSession) {
        let Some(engine) = session.engine.as_ref() else { return };
        let outcomes = engine.outcomes();
        let racers = outcomes.len();
        let me = outcomes[0];
        let humans: Vec<Outcome> = outcomes.iter().copied().filter(|o| engine.racers()[o.racer].human).collect();
        // A player who did not finish (out, or the time ran out) counts as last.
        let place = if me.finished { me.place } else { racers as u32 };
        session.result = Some(RaceResult { place, racers, time: me.time });
        if let (true, Some(time), false) =
            (me.finished, me.time, session.kind == nfsmw_data::gameplay::RaceType::SpeedTrap)
        {
            crate::frontend::custom_race::records::note_time(&session.event, time);
            crate::frontend::custom_race::records::note_lap(&session.event, time - session.lap_mark);
        }
        session.stage = Stage::Results { left: RACE_OVER_SECS };
        self.note_race_outcome(session);
        session.banner = Banner::default();
        log::info!(
            "race: {} over: place {place}/{racers}{}",
            session.name,
            me.time.map_or_else(String::new, |t| format!(", {t:.1} s"))
        );
        if let Some(drive) = self.players.lead_mut() {
            drive.hold_on_grid(false);
            drive.set_steer_assist(None);
        }
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.release_racers();
            for id in 1..racers {
                if let Some(state) = traffic.racer_mut(id) {
                    state.set_goal(None);
                }
            }
            // The cops of a Heat Level give up at the line, as the race's chase is over with it (`heat.rs`).
            if session.heat_chase.is_some() {
                traffic.stop_pursuit();
            }
        }
        if session.options.career {
            // A co-op race credits the career with the best human place; the result card keeps player 1's own.
            let credited = match self.coop.is_some() {
                true => coop_place(&humans, racers),
                false => u8::try_from(place).unwrap_or(u8::MAX),
            };
            self.career_race_finished(&session.event, credited, session.cash_value);
            return;
        }
        // Outside the career a win of the challenge series opens the events it lists. Running out of time alone on
        // the road is last place of one racer, not a win.
        if me.finished && place == 1 {
            crate::frontend::challenge_series::record_win(&session.event);
        }
    }

    /// Takes the race down: the racers leave, the traffic count is given back and the car is let go.
    pub(super) fn end_race(&mut self, mut session: RaceSession) {
        self.release_race(&mut session);
        self.race_hold = false;
        self.end_split();
        self.unpark_guest_after_race();
        log::info!("race: {} ended", session.name);
        if session.options.career {
            self.career_race_left();
        }
    }

    /// What a race holds on the world, given back: the racers, the race's walls, the traffic count a console command had
    /// set, the grid hold and the drag steering. The career hears nothing: `end_race` adds its roaming autosave, a restart adds none.
    pub(super) fn release_race(&mut self, session: &mut RaceSession) {
        // A race given up in its intro or outro takes the movie or NIS it plays with it.
        if matches!(session.stage, Stage::Intro { .. } | Stage::Outro { .. }) {
            self.stop_nis();
            self.stop_story_movie();
        }
        self.lower_race_walls(std::mem::take(&mut session.walls));
        // The stage's region gates come back after the race's own walls are down.
        self.restore_region_gates();
        self.finish_flares.clear();
        self.end_photo_shot();
        if let Some(traffic) = self.traffic.as_mut() {
            traffic.clear_racers();
        }
        self.traffic_manual = session.previous_traffic;
        if let Some(drive) = self.players.lead_mut() {
            drive.hold_on_grid(false);
            drive.set_steer_assist(None);
        }
    }

    /// Ends the race now, with no result (the console's `race stop`, or the world going away).
    pub(in crate::scenes::world) fn abandon_race(&mut self) -> bool {
        let Some(session) = self.race.take() else { return false };
        self.end_race(session);
        true
    }
}
