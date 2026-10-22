//! Custom Race, step three: the options of the race (`Track_Options.fng`, the original's `UIQRTrackOptions`). The package is an
//! option list like the settings pages, so the rows run on [`WidgetMenu`]; the rows and their values are `options.rs`. Back
//! returns to the track list, Accept goes on to the car select, and Defaults (key 1) asks whether to put the options back.
//! Spec: `docs/specs/custom-race.md` section 4.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK, PAD_BUTTON0};
use glam::Vec2;

use super::state;
use crate::frontend::confirm::Confirm;
use crate::frontend::ids::{self, screen};
use crate::frontend::logic::{Args, Category, Command, Cx, Footer, Pick, ScreenLogic};
use crate::frontend::widget_menu::WidgetMenu;
use crate::ui::Text;

/// Defaults is the fifth face button; the key `1` presses it and the second one, which the package also reports.
const DEFAULTS: u32 = PAD_BUTTON0 + 5;
/// `Are you sure you want to reset to defaults? Previous settings will be lost.`
const RESET_QUESTION: u32 = 0x8AEF_5AE8;

pub struct TrackOptions {
    menu: WidgetMenu,
}

impl Default for TrackOptions {
    fn default() -> Self {
        Self { menu: WidgetMenu::new(Args { category: Category::CustomRace, ..Args::default() }) }
    }
}

impl ScreenLogic for TrackOptions {
    fn start(&mut self, cx: &mut Cx) {
        self.menu.start(cx);
        // The header is a string of its own in this package: `<mode> options`.
        let title = state::mode().options_title();
        if let Some(header) = cx.object(ids::HEADER_TEXT) {
            cx.label(header, title);
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_BACK => cx.send(Command::Switch(screen::TRACK_SELECT.into(), Args::default())),
            PAD_ACCEPT => self.accept(cx),
            DEFAULTS => cx.send(Command::Confirm(Confirm::new(Text::Game(RESET_QUESTION), Command::CustomDefaults))),
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

    fn footer(&self) -> Footer {
        Footer::Defaults
    }
}

impl TrackOptions {
    /// Accept: the options are kept in [`state`] already; the car select comes next, or in split screen Press Start for player 1.
    /// Nothing happens without a picked track.
    fn accept(&self, cx: &mut Cx) {
        if state::with(|s| s.event().is_none()) {
            return;
        }
        if state::with(|s| s.split.on) {
            state::with(|s| s.split.clear_seats());
            return cx.send(Command::Switch(screen::PRESS_START.into(), Args::default()));
        }
        cx.send(Command::Switch(screen::CAR_SELECT.into(), Args { custom: true, ..Args::default() }));
    }
}
