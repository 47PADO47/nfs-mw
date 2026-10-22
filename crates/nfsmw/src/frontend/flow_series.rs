//! The challenge series in the flow: starting an event of the series, and the series screen over the menu backdrop when a
//! race started from it ends (`challenge_series/`).

use super::*;

/// What a race started from the challenge series asks of the flow.
#[derive(Default)]
pub(in crate::frontend) struct Series {
    /// Leaving the race opens the series screen again, not the main menu.
    pub active: bool,
}

impl Frontend {
    /// Starts an event of the series with the transmission the player chose; the original stores the choice with the
    /// player's settings.
    pub(in crate::frontend) fn start_challenge(
        &mut self,
        race: String,
        transmission: crate::settings::Transmission,
        host: &mut Host,
        env: &mut Env,
    ) {
        env.settings.transmission = transmission;
        env.changed.transmission = Some(transmission);
        self.career.end_play();
        self.origin = crate::frontend::logic::QuitTo::Main;
        self.series = Series { active: true };
        self.race_options = Some(Default::default());
        self.quick_race = Some(race);
        self.start_free_roam(LoadingScreen::Plain, host, env);
    }

    /// The challenge series over the menu backdrop, where a race started from it ends (`from_series`).
    pub(in crate::frontend) fn show_series(&mut self, host: &mut Host, env: &mut Env) {
        self.series.active = false;
        self.show_menu_at(crate::frontend::ids::screen::CHALLENGE_SERIES, Args::default(), host, env);
    }
}
