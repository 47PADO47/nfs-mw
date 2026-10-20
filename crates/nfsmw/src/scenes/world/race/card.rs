//! The race card for the HUD: the place, the lap, the clocks and the banner, built from the race each frame.

use super::{RaceSession, Stage};
use crate::hud::{RaceCard, ResultCard};
use crate::scenes::world::WorldScene;

/// The banner fades out over its last half second.
const BANNER_FADE: f32 = 0.5;

impl WorldScene {
    /// The card of the race being run, if any. Nothing while the grid is still loading.
    pub(in crate::scenes::world) fn race_card(&self) -> Option<RaceCard> {
        let session = self.race.as_ref()?;
        let engine = session.engine.as_ref()?;
        let places = engine.places();
        let laps = engine.course().laps;
        let me = engine.racers().first()?;
        Some(RaceCard {
            place: places[0],
            racers: places.len(),
            lap: (laps > 1).then(|| ((me.progress.laps + 1).min(laps), laps)),
            elapsed: engine.elapsed(),
            time_left: engine.has_time_limit().then(|| engine.time_left()),
            banner: banner(session),
            points: engine.config().rank_by_points.then(|| engine.standings()[0].points),
            result: session.result.map(|r| ResultCard { place: r.place, racers: r.racers, time: r.time }),
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
    if matches!(session.stage, Stage::Results { .. }) || session.banner.left <= 0.0 {
        return None;
    }
    Some((session.banner.text.clone(), (session.banner.left / BANNER_FADE).min(1.0)))
}
