//! Quick Race, step two: the track's options (`UIQRTrackOptions`, `Track_Options.fng`). The package is an option list
//! like the settings pages, so the rows (laps, opponents, difficulty, traffic) run on [`WidgetMenu`] with the rows of
//! `quick_options.rs`. Back returns to the track list. Accept plays the package's own leave animation and starts the race
//! on the track that was picked, with the chosen options: the world opens behind the loading screen and the race begins on
//! the grid (`quick_race.rs`, docs/specs/frontend-menus.md, section 5).

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};
use glam::Vec2;

use super::ids::{self, screen};
use super::logic::{Args, Category, Command, Cx, Pick, ScreenLogic};
use super::quick_race;
use super::widget_menu::WidgetMenu;

pub struct TrackOptions {
    menu: WidgetMenu,
}

impl Default for TrackOptions {
    fn default() -> Self {
        Self { menu: WidgetMenu::new(Args { category: Category::QuickRace, ..Args::default() }) }
    }
}

impl ScreenLogic for TrackOptions {
    fn start(&mut self, cx: &mut Cx) {
        self.menu.start(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => cx.send(Command::Switch(screen::TRACK_SELECT.into(), Args::default())),
            PAD_ACCEPT => start(cx),
            // The list's own leave would go back to the option categories.
            ids::EXIT_COMPLETE => {}
            _ => self.menu.message(cx, message),
        }
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        self.menu.click(cx, point)
    }

    fn vertical(&self) -> bool {
        self.menu.vertical()
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        self.menu.pick_row(cx, row, pick);
    }

    fn list(&self, assets: &crate::ui::UiAssets) -> Option<crate::ui::theme::MenuList> {
        self.menu.list(assets)
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        self.menu.scroll(cx, steps);
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.menu.tick(cx, dt);
    }
}

/// Asks the front end to start the picked track. Nothing happens while the track list is not read yet.
fn start(cx: &mut Cx) {
    let Some(board) = quick_race::board() else { return };
    let Some(track) = quick_race::picked(&board) else { return };
    cx.send(Command::StartQuickRace(track.name.clone()));
}
