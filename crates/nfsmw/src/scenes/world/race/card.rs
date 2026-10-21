//! The race card for the HUD: the place, the lap, the clocks and the banner, built from the race each frame.

use super::Stage;
use crate::hud::{Count, LeaderEntry, LeaderRow, Leaders, Message, RaceCard, RaceHud, ResultCard};
use crate::scenes::world::WorldScene;
use nfsmw_data::gameplay::RaceType;

/// The serial of the race-over line: past any banner's.
const RESULT_SERIAL: u32 = 1 << 30;

impl WorldScene {
    /// The card of the race being run: what the HUD package has no object for (the points of a speed-trap race).
    pub(in crate::scenes::world) fn race_card(&self) -> Option<RaceCard> {
        let session = self.race.as_ref()?;
        let engine = session.engine.as_ref()?;
        let points = engine.config().rank_by_points.then(|| engine.standings()[0].points)?;
        matches!(session.stage, Stage::Running).then_some(RaceCard { points: Some(points) })
    }

    /// The line the HUD's generic message group shows: the banner (a checkpoint, a lap, a bonus), or the race-over message
    /// while it runs. The countdown numbers are the countdown group's.
    pub(in crate::scenes::world) fn race_message(&self) -> Option<Message> {
        let session = self.race.as_ref()?;
        if let (Stage::Results { .. }, Some(result)) = (&session.stage, session.result) {
            let card = ResultCard {
                place: result.place,
                racers: result.racers,
                time: result.time,
                challenge: session.challenge.is_some(),
            };
            // One line for the whole message: its serial is past every banner's.
            return Some(Message { serial: RESULT_SERIAL, text: card.title() });
        }
        let banner = &session.banner;
        let counting = Count::from_banner(&banner.text).is_some();
        if matches!(session.stage, Stage::Results { .. } | Stage::PostRace { .. }) || banner.left <= 0.0 || counting {
            return None;
        }
        Some(Message { serial: banner.serial, text: banner.text.clone() })
    }

    /// What the HUD package shows of the race: the place, lap, clock, progress and the countdown step.
    pub(in crate::scenes::world) fn race_hud(&self) -> Option<RaceHud> {
        let session = self.race.as_ref()?;
        if matches!(session.stage, Stage::Intro { .. }) {
            return None;
        }
        if session.result.is_some() && !matches!(session.stage, Stage::Results { .. }) {
            return None;
        }
        let engine = session.engine.as_ref()?;
        let places = engine.places();
        let course = engine.course();
        let me = engine.racers().first()?;
        let laps = course.laps.max(1);
        let count = (session.banner.left > 0.0).then(|| Count::from_banner(&session.banner.text)).flatten();
        // A circuit and a knockout say the lap, the others how much of the race is done (the original picks by race type).
        let lapped = matches!(session.kind, RaceType::Circuit | RaceType::Knockout) || course.laps > 1;
        Some(RaceHud {
            place: places[0],
            racers: places.len() as u32,
            lap: lapped.then(|| ((me.progress.laps + 1).min(laps), laps)),
            clock: if engine.has_time_limit() { engine.time_left() } else { engine.elapsed() },
            progress: me.percent / 100.0,
            count,
        })
    }

    /// The leaderboard of the HUD package: the four best placed racers with their progress (`hud/leaders.rs`). Shown from
    /// the countdown to the race-over message, in every race type with more than one racer.
    pub(in crate::scenes::world) fn race_leaders(&self) -> Option<Leaders> {
        let session = self.race.as_ref()?;
        if matches!(session.stage, Stage::Intro { .. }) {
            return None;
        }
        if session.result.is_some() && !matches!(session.stage, Stage::Results { .. }) {
            return None;
        }
        let engine = session.engine.as_ref()?;
        let places = engine.places();
        let alias = crate::career::CareerHandle::shared().with_loaded(|career| career.alias.clone());
        let entries = engine.racers().iter().enumerate().map(|(i, racer)| {
            let name = match (i, &alias) {
                (0, Some(alias)) => alias.clone(),
                _ => session.plan.get(i).map_or_else(String::new, |p| p.name.clone()),
            };
            let points = racer.traps.total();
            LeaderEntry {
                place: places[i],
                player: i == 0,
                row: LeaderRow { name, percent: racer.percent, points, out: racer.out_at.is_some(), busted: false },
            }
        });
        Some(Leaders::pick(entries.collect(), engine.course().length(), engine.config().rank_by_points))
    }

    /// Draws the race card over the frame.
    pub(in crate::scenes::world) fn show_race_card(&self, ctx: &egui::Context) {
        if let Some(card) = self.race_card() {
            crate::hud::show_race_card(ctx, &card);
        }
    }
}
