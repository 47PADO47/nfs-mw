//! Movies over the menus: the tutorials and the story movies (`MOVIES/<name>`), played full screen over the parked menu
//! backdrop, and the step the flow takes when one is over or skipped (`AfterMovie`).

use super::*;
use crate::input::{Action, ActionState};

/// What the flow does once a movie is over or skipped.
#[derive(Clone, Debug, PartialEq)]
pub enum AfterMovie {
    /// Opens this screen over the menu backdrop it wants, with these arguments. A tutorial opens the screen that asked
    /// for it, where it was left.
    Open(String, Args),
    /// Carries out this command, as a screen's own command would be (`Frontend::apply`).
    #[allow(dead_code)] // no caller yet; the career sends its story movies with it
    Command(Box<Command>),
}

impl Frontend {
    /// Plays the movie `MOVIES/<name>` full screen over the parked menu backdrop; `then` is what the flow does when it is
    /// over. A movie that does not open is logged and nothing changes.
    pub(in crate::frontend) fn play_movie(&mut self, name: &str, then: AfterMovie, host: &mut Host) {
        let scene = match crate::movie::MovieScene::open(&self.dir, name, 0.0) {
            Ok(scene) => scene,
            Err(e) => return log::warn!("the movie {name} is not played: {e:#}"),
        };
        self.latch = true;
        self.screens.clear();
        self.stage = Stage::Movie { then };
        self.replace_scene(host, Box::new(scene), true);
    }

    /// One frame of a movie over the menus: the movie is over when it has ended, or when the pad's accept, start or back
    /// or a click skips it.
    pub(in crate::frontend) fn movie_frame(&mut self, actions: &ActionState, host: &mut Host, env: &mut Env) {
        let skip = actions.just_pressed(Action::MenuAccept)
            || actions.just_pressed(Action::MenuStart)
            || actions.just_pressed(Action::MenuBack)
            || actions.just_pressed(Action::Click);
        if host.scene.finished() || skip {
            self.end_movie(host, env);
        }
    }

    /// The movie is over (or skipped): the flow takes the step it was given. Nothing happens unless a movie is playing.
    pub(in crate::frontend) fn end_movie(&mut self, host: &mut Host, env: &mut Env) {
        let Stage::Movie { then } = self.stage.clone() else { return };
        self.stage = Stage::Menus;
        log::info!("movie over");
        self.latch = true;
        self.paused.set(false);
        match then {
            AfterMovie::Open(screen, args) => self.open_menu(&screen, args, host, env),
            AfterMovie::Command(command) => self.apply(vec![*command], host, env),
        }
    }
}
