//! Which developer panel is up, and the ways in and out of them that need no keyboard: the devtools menu, the vehicle panel (F11),
//! a controller's back button, and the `devtools` console command (`docs/devtools-vehicle.md`).
//!
//! The console, the graphics lab, the extra options and the garage overlay keep their own state and their own keys (F12, F8, F5,
//! F6); this module only closes and opens them, one at a time, so a button can do what the keys do. The menu and the vehicle panel
//! are this module's own.

use bevy_app::{App, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_input::{ButtonInput, InputSystems, keyboard::KeyCode};

use super::{Console, GraphicsLab};
use crate::app::Host;
use crate::input::{MouseCapture, UiFocus};

/// A developer panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Panel {
    Menu,
    Console,
    GraphicsLab,
    ExtraOptions,
    Garage,
    Vehicle,
}

impl Panel {
    pub(crate) const ALL: [Self; 6] =
        [Self::Menu, Self::Console, Self::GraphicsLab, Self::ExtraOptions, Self::Garage, Self::Vehicle];

    /// What the panel is called on the menu.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Menu => "Devtools menu",
            Self::Console => "Console",
            Self::GraphicsLab => "Graphics lab",
            Self::ExtraOptions => "Extra options",
            Self::Garage => "Garage",
            Self::Vehicle => "Vehicle",
        }
    }

    /// The key that opens it.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Menu => "L3+R3",
            Self::Console => "F12",
            Self::GraphicsLab => "F8",
            Self::ExtraOptions => "F5",
            Self::Garage => "F6",
            Self::Vehicle => "F11",
        }
    }

    /// The word the `devtools` console command takes.
    pub(crate) fn word(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Console => "console",
            Self::GraphicsLab => "lab",
            Self::ExtraOptions => "extra",
            Self::Garage => "garage",
            Self::Vehicle => "vehicle",
        }
    }

    pub(crate) fn of_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|panel| panel.word() == word)
    }
}

/// What asks the panels to change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Request {
    /// Open a panel and close the others; `from_menu`: back returns to the menu.
    Open { panel: Panel, from_menu: bool },
    /// Close every panel.
    CloseAll,
    /// The controller's chord: close everything if anything is up, else open the menu.
    Toggle,
    /// The controller's back button, with nothing smaller to close first: leave the panel for the menu, or the menu for the game.
    Back,
}

/// The state of the devtools menu and the vehicle panel, and what the controller feeds the panels.
#[derive(Resource, Default)]
pub(crate) struct DevPanels {
    pub menu: bool,
    pub vehicle: bool,
    /// The vehicle panel's own state: its message line.
    pub message: Option<String>,
    /// The panel the menu opened: back returns to the menu.
    from_menu: bool,
    active: bool,
    restore_capture: bool,
    requests: Vec<Request>,
    /// The egui events the controller made since the last pass (`pad_nav`).
    pub events: Vec<egui::Event>,
    /// A controller is connected, so the panels show its buttons.
    pub pad: bool,
    /// Buttons pressed by the `devtools press` console command, served like the controller's own.
    pub pressed: Vec<super::pad_nav::Button>,
}

impl DevPanels {
    pub(crate) fn request(&mut self, request: Request) {
        self.requests.push(request);
    }

    /// Whether the menu or the vehicle panel is up.
    pub(crate) fn any(&self) -> bool {
        self.menu || self.vehicle
    }
}

/// Everything that decides which panel is up.
#[derive(SystemParam)]
pub(super) struct Panels<'w> {
    pub state: ResMut<'w, DevPanels>,
    pub console: ResMut<'w, Console>,
    pub lab: ResMut<'w, GraphicsLab>,
    pub extra: ResMut<'w, super::extra_options::Overlay>,
    pub garage: ResMut<'w, crate::garage::Overlay>,
    pub capture: ResMut<'w, MouseCapture>,
    pub focus: ResMut<'w, UiFocus>,
    pub host: NonSendMut<'w, Host>,
}

impl Panels<'_> {
    /// The panel in front, if any.
    pub(crate) fn which(&self) -> Option<Panel> {
        let up = [
            (self.state.menu, Panel::Menu),
            (self.state.vehicle, Panel::Vehicle),
            (self.console.open, Panel::Console),
            (self.lab.open, Panel::GraphicsLab),
            (self.extra.open, Panel::ExtraOptions),
            (self.garage.open, Panel::Garage),
        ];
        up.into_iter().find(|(open, _)| *open).map(|(_, panel)| panel)
    }

    fn close_all(&mut self) {
        self.state.menu = false;
        self.state.vehicle = false;
        self.state.from_menu = false;
        self.console.set_open(false, &mut self.capture, &mut self.focus);
        self.lab.close(&mut self.capture, &mut self.focus);
        self.extra.open = false;
        self.garage.set_open(false, &mut self.capture, &mut self.focus);
    }

    fn open(&mut self, panel: Panel, from_menu: bool) {
        self.close_all();
        self.state.from_menu = from_menu;
        match panel {
            Panel::Menu => self.state.menu = true,
            Panel::Vehicle => self.state.vehicle = true,
            Panel::Console => self.console.set_open(true, &mut self.capture, &mut self.focus),
            Panel::GraphicsLab => self.lab.open_from_menu(&mut self.capture, &mut self.focus),
            Panel::ExtraOptions => {
                self.extra.open = true;
                if let Some(state) = self.host.scene.extra_options() {
                    state.hide_ui = false;
                }
            }
            Panel::Garage => self.open_garage(),
        }
    }

    fn open_garage(&mut self) {
        if self.host.scene.paused() {
            return;
        }
        let Some(model) = self.host.scene.garage() else { return };
        model.reset_draft();
        self.garage.menu = false;
        self.garage.set_open(true, &mut self.capture, &mut self.focus);
    }

    fn back(&mut self) {
        let Some(was) = self.which() else { return };
        let return_to_menu = self.state.from_menu && was != Panel::Menu;
        self.close_all();
        if return_to_menu {
            self.open(Panel::Menu, false);
        }
    }

    /// One frame: the requests, then who has the keyboard and the mouse.
    pub(crate) fn run(&mut self) {
        // A normal shop owns its transaction: the tools stay out of it.
        let shop = self.garage.menu;
        for request in std::mem::take(&mut self.state.requests) {
            if shop {
                continue;
            }
            match request {
                Request::Open { panel, from_menu } => self.open(panel, from_menu),
                Request::CloseAll => self.close_all(),
                Request::Toggle => match self.which() {
                    Some(_) => self.close_all(),
                    None => self.open(Panel::Menu, false),
                },
                Request::Back => self.back(),
            }
        }
        // A key opened a panel of its own over the menu or the vehicle panel: they give way.
        let other = self.console.open || self.lab.open || self.extra.open || self.garage.open;
        if other && self.state.any() {
            self.state.menu = false;
            self.state.vehicle = false;
        }
        self.sync_focus(other);
    }

    /// The menu and the vehicle panel take the keyboard and the mouse while they are up, and give them back when they go.
    fn sync_focus(&mut self, other_open: bool) {
        if self.state.any() {
            if !self.state.active {
                self.state.active = true;
                self.state.restore_capture = self.capture.0;
            }
            self.capture.0 = false;
            self.focus.0 = true;
            return;
        }
        if !self.state.active {
            return;
        }
        self.state.active = false;
        self.capture.0 = self.state.restore_capture;
        self.focus.0 = other_open;
    }
}

/// F11 opens and closes the vehicle panel; the requests of the controller and the console are served.
fn apply(mut panels: Panels, mut keys: ResMut<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F11) {
        keys.reset(KeyCode::F11);
        let request = match panels.state.vehicle {
            true => Request::CloseAll,
            false => Request::Open { panel: Panel::Vehicle, from_menu: false },
        };
        panels.state.request(request);
    }
    panels.run();
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<DevPanels>().add_systems(
        PreUpdate,
        (super::pad_nav::collect, apply)
            .chain()
            .after(InputSystems)
            .after(super::extra_options::toggle)
            .before(crate::input::update_actions),
    );
}

#[cfg(test)]
#[path = "panels_tests.rs"]
mod tests;
