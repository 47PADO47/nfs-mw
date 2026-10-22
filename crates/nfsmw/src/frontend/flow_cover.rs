//! The loading screen over a race that starts in a world that is already up (`docs/specs/loading-screen.md` section 6).
//! A race of the career's chain (the next prologue race, a boss approach), a race asked for by the front end and a retry
//! put the player's car on a new start while the scenery around it is not streamed yet; without a cover the player sees
//! the empty world between one cinematic and the next. The original keeps its loading screen up for the whole change.

use super::*;

impl Frontend {
    /// Every frame of driving: a race that has been asked for, or waits for its grid, is covered by the loading screen until
    /// the grid is on. A restart from the pause menu or the results stays on the same start and is not covered.
    pub(in crate::frontend) fn poll_race_cover(&mut self, host: &mut Host, env: &mut Env) {
        if self.stage != Stage::Playing {
            return;
        }
        if !host.scene.race_staging() {
            self.restarting = false;
            return;
        }
        if self.restarting {
            return;
        }
        self.cover_race_start(host, env);
    }

    /// Puts the loading screen over the world, concealed, until the world says the race stands on its grid
    /// (`loading_frame`). The career shows its tips; the other races the plain screen.
    fn cover_race_start(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("race: the loading screen covers the start");
        let screen = match self.origin {
            QuitTo::Career => LoadingScreen::Tips,
            _ => LoadingScreen::Plain,
        };
        self.latch = true;
        self.paused.set(false);
        self.conceal.set(true);
        self.screens.clear();
        self.load = Some(Load::Streaming);
        self.stage = Stage::Loading;
        let commands = self.screens.open(screen.file(), Args::default(), true, env);
        self.apply(commands, host, env);
    }
}
