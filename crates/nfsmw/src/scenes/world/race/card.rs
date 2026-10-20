//! The race card for the HUD: the place, the lap, the clocks and the banner, built from the race each frame.

use super::{RaceSession, Stage};
use crate::hud::{Count, RaceCard, RaceHud, ResultCard};
use crate::scenes::world::WorldScene;

/// The banner fades out over its last half second.
const BANNER_FADE: f32 = 0.5;

impl WorldScene {
    /// The card of the race being run, if any. Nothing while the grid is still loading.
    pub(in crate::scenes::world) fn race_card(&self) -> Option<RaceCard> {
        let session = self.race.as_ref()?;
        let engine = session.engine.as_ref();
        // A pursuit challenge has no engine: its card is the banner and the result.
        if engine.is_none() && session.challenge.is_none() {
            return None;
        }
        Some(RaceCard {
            banner: banner(session),
            points: engine.and_then(|e| e.config().rank_by_points.then(|| e.standings()[0].points)),
            result: session.result.filter(|_| matches!(session.stage, Stage::Results { .. })).map(|r| ResultCard {
                place: r.place,
                racers: r.racers,
                time: r.time,
                challenge: session.challenge.is_some(),
            }),
        })
    }

    /// What the HUD package shows of the race: the place, lap, clock, progress and the countdown step.
    pub(in crate::scenes::world) fn race_hud(&self) -> Option<RaceHud> {
        let session = self.race.as_ref()?;
        if session.result.is_some() {
            return None;
        }
        let engine = session.engine.as_ref()?;
        let places = engine.places();
        let course = engine.course();
        let me = engine.racers().first()?;
        let gates = course.checkpoints.len().max(1) as f32;
        let laps = course.laps.max(1);
        let done = me.progress.laps as f32 * gates + me.progress.reached as f32;
        let count = (session.banner.left > 0.0).then(|| Count::from_banner(&session.banner.text)).flatten();
        Some(RaceHud {
            place: places[0],
            racers: places.len() as u32,
            lap: (course.laps > 1).then(|| ((me.progress.laps + 1).min(laps), laps)),
            clock: if engine.has_time_limit() { engine.time_left() } else { engine.elapsed() },
            progress: done / (gates * laps as f32),
            count,
        })
    }

    /// Draws the race card over the frame.
    pub(in crate::scenes::world) fn show_race_card(&self, ctx: &egui::Context) {
        if let Some(card) = self.race_card() {
            crate::hud::show_race_card(ctx, &card);
        }
    }
}

/// The banner's text and opacity.
fn banner(session: &RaceSession) -> Option<(String, f32)> {
    // The countdown is the HUD package's own group (`race_hud`); the card keeps the other lines.
    let counting = Count::from_banner(&session.banner.text).is_some();
    if matches!(session.stage, Stage::Results { .. } | Stage::PostRace { .. }) || session.banner.left <= 0.0 || counting
    {
        return None;
    }
    Some((session.banner.text.clone(), (session.banner.left / BANNER_FADE).min(1.0)))
}
