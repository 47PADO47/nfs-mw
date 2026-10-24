//! What a LAN page works with for one key or one frame, and what it can ask of the front end.

use super::field::Typed;
use crate::frontend::lan::NetFrontend;
use crate::settings::Settings;

/// A key the LAN pages take: the menu pad, a typed key, or a click on a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanKey {
    Up,
    Down,
    Left,
    Right,
    Accept,
    Back,
    Typed(Typed),
    /// A click on row `n`: it is selected and accepted.
    Click(usize),
    /// A click on a row's chevron: it is selected and stepped.
    Step(usize, bool),
}

/// What a page asks of the front end around it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuOut {
    /// Close the LAN menu and go back to the main menu.
    Close,
    /// The player's name was changed: keep it in the settings and the config file.
    SaveName(crate::settings::net::NetName),
    /// Open the car select (`Car_Select.fng`); its pick comes back with [`super::LanMenu::car_chosen`].
    CarSelect,
}

/// The network, the settings and the cars the pages read, and the answers they give.
pub struct Cx<'a> {
    pub net: &'a mut dyn NetFrontend,
    pub settings: &'a Settings,
    /// The car folders the game knows, to name the cars of the roster.
    pub cars: &'a [String],
    pub out: Vec<MenuOut>,
}

impl<'a> Cx<'a> {
    pub fn new(net: &'a mut dyn NetFrontend, settings: &'a Settings, cars: &'a [String]) -> Self {
        Self { net, settings, cars, out: Vec::new() }
    }
}
