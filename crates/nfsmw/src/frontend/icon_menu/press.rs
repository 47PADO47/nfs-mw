//! What a press on an icon does: the package's leave (or none, in a vertical menu), then the screen the icon opens.

use super::{Args, Category, Command, Confirm, Cx, IconMenu, Id, Kind, Pending, ids, safehouse_menu, screen};
use crate::frontend::custom_race::state as custom_state;
use crate::frontend::logic::Sub;

impl IconMenu {
    /// Posts the leave message: the package plays its leave scripts and sends `EXIT_COMPLETE` when they are done.
    pub(super) fn leave(&mut self, cx: &mut Cx, pending: Pending) {
        if !self.reacts {
            return;
        }
        if self.vertical {
            return self.act(cx, pending);
        }
        self.pending = Some(pending);
        self.reacts = false;
        cx.rt.post_to_package(cx.package, ids::LEAVE_SCREEN);
    }

    /// Plays the package's own leave timing (the event handler's forward script) instead of the leave message.
    pub(super) fn leave_by_script(&mut self, cx: &mut Cx, pending: Pending) {
        if !self.reacts {
            return;
        }
        if self.vertical {
            return self.act(cx, pending);
        }
        self.pending = Some(pending);
        self.reacts = false;
        cx.script(ids::EVENT_HANDLER, ids::SCRIPT_FORWARD);
    }

    /// The screen has left: do what was pressed.
    pub(super) fn act(&mut self, cx: &mut Cx, pending: Pending) {
        let pause = self.pause();
        let race = self.args.pause_ctx.race;
        match (self.kind, pending) {
            (Kind::Main, Pending::Pressed(Id::MyCars)) => cx.send(Command::OpenGarage),
            (Kind::Main, Pending::Pressed(Id::Career)) => {
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { sub: Sub::Career, ..self.args }));
            }
            (Kind::Main, Pending::Pressed(Id::Challenge)) => {
                cx.send(Command::Switch(screen::CHALLENGE_SERIES.into(), Args::default()));
            }
            (Kind::Main, Pending::Pressed(Id::QuickRace)) => {
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { sub: Sub::QuickRace, ..self.args }));
            }
            (Kind::Main, Pending::Pressed(Id::Options)) => {
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { options: true, ..self.args }));
            }
            // The LAN menu is an egui panel over the main menu, which stays as it is under it (`lan/link.rs`).
            (Kind::Main, Pending::Pressed(Id::LanPlay)) => crate::frontend::lan::link::request_open(),
            (Kind::Main, Pending::Pressed(Id::AliasManager)) => {
                cx.send(Command::Switch(screen::MC_PROFILE_MANAGER.into(), self.args));
            }
            (Kind::Career, Pending::Pressed(id)) => crate::frontend::career_screens::career_menu::press(cx, id),
            (Kind::QuickRace, Pending::Pressed(Id::SplitScreen)) => {
                custom_state::with(|state| state.split.begin());
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { sub: Sub::CustomRace, ..self.args }));
            }
            (Kind::QuickRace, Pending::Pressed(Id::QuickPlay)) => {
                custom_state::with(|state| state.split.end());
                cx.send(Command::Switch(screen::QUICK_RACE_BRIEF.into(), self.args));
            }
            (Kind::QuickRace, Pending::Pressed(Id::CustomRace)) => {
                custom_state::with(|state| state.split.end());
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { sub: Sub::CustomRace, ..self.args }));
            }
            (Kind::CustomRace, Pending::Pressed(Id::CustomMode(mode))) => {
                custom_state::with(|state| state.choose_mode(mode));
                cx.send(Command::Switch(screen::TRACK_SELECT.into(), Args::default()));
            }
            (Kind::AliasManager, Pending::Pressed(id)) => crate::frontend::alias_menu::press(cx, id),
            (Kind::CustomRace, Pending::Back) => {
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { sub: Sub::QuickRace, ..self.args }));
            }
            (Kind::Career | Kind::QuickRace | Kind::AliasManager, Pending::Back) => {
                cx.send(Command::Switch(screen::MAIN_MENU.into(), Args::default()));
            }
            (Kind::Categories { .. }, Pending::Pressed(Id::Category(category))) => self.open_options(cx, category),
            (Kind::Categories { .. }, Pending::Pressed(Id::Controls)) => self.open_controls(cx),
            (Kind::Categories { .. }, Pending::Pressed(Id::EaTrax)) => self.open_jukebox(cx),
            (Kind::Categories { .. }, Pending::Pressed(Id::Credits)) => self.open_credits(cx),
            (Kind::Categories { .. }, Pending::Pressed(Id::ControlOptions)) => {
                self.open_options(cx, Category::Controls)
            }
            (Kind::Categories { .. }, Pending::Back) if pause => {
                cx.send(Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: false, ..self.args }));
            }
            (Kind::Categories { .. }, Pending::Back) => {
                cx.send(Command::Switch(screen::MAIN_MENU.into(), Args::default()));
            }
            (Kind::Categories { .. } | Kind::Pause, Pending::Start) | (Kind::Pause, Pending::Back) => {
                cx.send(Command::Resume);
            }
            (Kind::SafeHouse, Pending::Pressed(id)) => safehouse_menu::press(cx, self.args, id),
            (Kind::Pause, Pending::Pressed(Id::Resume | Id::ResumeRace)) => cx.send(Command::Resume),
            (Kind::Pause, Pending::Pressed(Id::PerformanceTuning)) => cx.send(Command::OpenTuning),
            (Kind::Pause, Pending::Pressed(Id::EndCoop)) => cx.send(Command::EndCoop),
            (Kind::Pause, Pending::Pressed(Id::JumpToSafeHouse)) => {
                let question = match self.args.pause_ctx.pursuit {
                    true => 0x15A1_B5A9,
                    false => 0x6925_D0BE,
                };
                cx.send(Command::Confirm(Confirm::new(crate::ui::Text::Game(question), Command::JumpToSafeHouse)));
            }
            (Kind::Pause, Pending::Pressed(Id::SafeHouseBlacklist(stage))) => {
                let args = Args { safehouse: true, pause: true, rival: stage, ..self.args };
                // The Remastered look has no overview page: its Blacklist is the list of rivals.
                let screen = match self.vertical {
                    true => screen::SAFE_HOUSE_RIVAL_CHALLENGE,
                    false => screen::SAFE_HOUSE_REPUTATION_OVERVIEW,
                };
                cx.send(Command::Switch(screen.into(), args));
            }
            (Kind::Pause, Pending::Pressed(Id::RestartRace)) => {
                cx.send(Command::Confirm(Confirm::restart_race(Command::RestartRace)));
            }
            (Kind::Pause, Pending::Pressed(Id::Quit(to))) => cx.send(Command::Confirm(Confirm::quit_to(to, race))),
            (Kind::Pause, Pending::Pressed(Id::Options)) => {
                cx.send(Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: true, ..self.args }));
            }
            _ => {}
        }
    }

    /// The tutorial movie of the selected kind of custom race, if it has one.
    pub(super) fn tutorial(&self) -> Option<&'static str> {
        match self.icons.get(self.current_icon()?)?.id {
            Id::CustomMode(mode) if self.kind == Kind::CustomRace => mode.tutorial(),
            _ => None,
        }
    }

    /// View Tutorial (key 2): plays the movie of the selected kind of race and comes back to this menu.
    pub(super) fn play_tutorial(&mut self, cx: &mut Cx) {
        let Some(movie) = self.tutorial().filter(|_| self.reacts) else { return };
        cx.memory.insert(self.memory_key(cx), self.current_icon().unwrap_or(0));
        cx.send(Command::PlayTutorial {
            movie: movie.to_owned(),
            back: screen::MAIN_MENU_SUB.to_owned(),
            args: Args { sub: Sub::CustomRace, ..self.args },
        });
    }

    /// Accept on the selected icon: the item's screen opens (after the package's leave, unless the menu is vertical).
    pub(super) fn press(&mut self, cx: &mut Cx) {
        if !self.reacts {
            return;
        }
        let Some(i) = self.current_icon() else { return };
        if !self.icons[i].enabled {
            // The package switched its buttons off for the leave animation that will not come.
            cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
            return;
        }
        cx.memory.insert(self.memory_key(cx), i);
        let id = self.icons[i].id;
        // Dialogs and profile lists overlay the retained menu. Do not run its leave scripts: No/Back must preserve its
        // header and brackets as well as its icons and input, as for the pause quit/restart questions.
        if (self.pause() && matches!(id, Id::Quit(_) | Id::RestartRace | Id::JumpToSafeHouse))
            || matches!(
                (self.kind, id),
                (Kind::SafeHouse, Id::SafeHouseResume)
                    | (Kind::AliasManager, Id::AliasCreate | Id::AliasLoad | Id::AliasSave | Id::AliasDelete)
                    | (Kind::Career, Id::CareerNew | Id::CareerLoad)
            )
        {
            self.act(cx, Pending::Pressed(id));
            return;
        }
        // LAN opens its panel over the main menu, which does not leave: its buttons go back on as for a greyed icon.
        if id == Id::LanPlay {
            cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
            self.act(cx, Pending::Pressed(id));
            return;
        }
        // The pause package has no leave message: its screens run the event handler's script themselves.
        if self.pause() {
            self.leave_by_script(cx, Pending::Pressed(id));
            return;
        }
        self.leave(cx, Pending::Pressed(id));
    }

    /// The option list of a category: the pause menu's in game, the main menu's otherwise.
    pub(super) fn open_options(&self, cx: &mut Cx, category: Category) {
        let name = if self.pause() { screen::PAUSE_OPTIONS } else { screen::OPTIONS };
        cx.send(Command::Switch(name.into(), Args { category, options: true, ..self.args }));
    }

    /// The key list (`controls_screen.rs`), which goes back to these categories when it is left.
    pub(super) fn open_controls(&self, cx: &mut Cx) {
        let name = if self.pause() { screen::PAUSE_CONTROLS } else { screen::CONTROLS };
        cx.send(Command::Switch(name.into(), Args { options: true, ..self.args }));
    }

    /// The EA Trax jukebox (`jukebox.rs`), which goes back to these categories when it is left.
    pub(super) fn open_jukebox(&self, cx: &mut Cx) {
        cx.send(Command::Switch(screen::EA_TRAX_JUKEBOX.into(), Args { options: true, ..self.args }));
    }

    /// The credits roll (`credits.rs`), which goes back to these categories when it is left.
    pub(super) fn open_credits(&self, cx: &mut Cx) {
        cx.send(Command::Switch(screen::CREDITS.into(), Args { options: true, ..self.args }));
    }
}
