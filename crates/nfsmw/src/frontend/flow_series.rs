//! The challenge series in the flow: a tutorial movie over the menus, and the series screen over the menu backdrop
//! when a race started from it ends (`challenge_series/`).

use super::*;

impl Frontend {
    /// Plays a tutorial movie full screen over the parked menu backdrop; `end_tutorial` brings the screen back.
    pub(super) fn play_tutorial(&mut self, name: &str, host: &mut Host) {
        let scene = match crate::movie::MovieScene::open(&self.dir, name, 0.0) {
            Ok(scene) => scene,
            Err(e) => return log::warn!("the tutorial {name} is not played: {e:#}"),
        };
        self.latch = true;
        self.screens.clear();
        self.stage = Stage::Tutorial;
        self.replace_scene(host, Box::new(scene), true);
    }

    /// The tutorial is over (or skipped): the challenge series screen opens again where it was left.
    pub(super) fn end_tutorial(&mut self, host: &mut Host, env: &mut Env) {
        log::info!("tutorial over");
        self.latch = true;
        self.paused.set(false);
        self.show_backdrop(host);
        let commands = self.screens.open(crate::frontend::ids::screen::CHALLENGE_SERIES, Args::default(), true, env);
        self.stage = Stage::Menus;
        self.apply(commands, host, env);
    }

    /// The challenge series over the menu backdrop, where a race started from it ends (`from_series`).
    pub(super) fn show_series(&mut self, host: &mut Host, env: &mut Env) {
        self.from_series = false;
        self.show_screen(crate::frontend::ids::screen::CHALLENGE_SERIES, host, env);
    }

    pub(super) fn show_screen(&mut self, name: &str, host: &mut Host, env: &mut Env) {
        log::info!("menu: {name}");
        self.latch = true;
        self.paused.set(false);
        self.screens.clear();
        self.show_backdrop(host);
        let commands = self.screens.open(name, Args::default(), true, env);
        self.stage = Stage::Menus;
        self.apply(commands, host, env);
    }
}
