//! What a press on an icon does: the package's leave (or none, in a vertical menu), then the screen the icon opens.

use super::{Args, Category, Command, Confirm, Cx, IconMenu, Id, Kind, Pending, ids, safehouse_menu, screen};

impl IconMenu {
    /// Posts the leave message: the package plays its leave scripts and sends `EXIT_COMPLETE` when they are done.
    pub(super) fn leave(&mut self, cx: &mut Cx, pending: Pending) {
        if self.vertical {
            return self.act(cx, pending);
        }
        self.pending = Some(pending);
        self.reacts = false;
        cx.rt.post_to_package(cx.package, ids::LEAVE_SCREEN);
    }

    /// Plays the package's own leave timing (the event handler's forward script) instead of the leave message.
    pub(super) fn leave_by_script(&mut self, cx: &mut Cx, pending: Pending) {
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
        match (self.kind, pending) {
            (Kind::Main, Pending::Pressed(Id::Career)) => cx.send(Command::StartCareer),
            (Kind::Main, Pending::Pressed(Id::Challenge)) => {
                cx.send(Command::Switch(screen::CHALLENGE_SERIES.into(), Args::default()));
            }
            (Kind::Main, Pending::Pressed(Id::QuickRace)) => {
                cx.send(Command::Switch(screen::TRACK_SELECT.into(), Args::default()));
            }
            (Kind::Main, Pending::Pressed(Id::Options)) => {
                cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), Args { options: true, ..self.args }));
            }
            (Kind::Main, Pending::Pressed(Id::AliasManager)) => {
                let args = Args { category: Category::AliasManager, options: true, ..self.args };
                cx.send(Command::Switch(screen::OPTIONS.into(), args));
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
            (Kind::Pause, Pending::Pressed(Id::Resume)) => cx.send(Command::Resume),
            (Kind::Pause, Pending::Pressed(Id::Options)) => {
                cx.send(Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: true, ..self.args }));
            }
            _ => {}
        }
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
        // Quit asks at once, without the pause menu leaving: its icons stay as they are, so the menu still
        // reacts when the question is answered No or dismissed.
        if self.pause() && matches!(id, Id::QuitToMenu) {
            cx.send(Command::Confirm(Confirm::quit_to_menu()));
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
