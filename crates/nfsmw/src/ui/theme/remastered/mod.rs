//! The Remastered theme: its own cluster for the drive readouts. It fills the tachometer, the speedometer, the gear and the
//! nitrous bar, and it hides the speed breaker's bar, so the original ones do not draw. Its icon menus are vertical lists. Every other slot falls back to
//! Original.
//!
//! - [`layout`]: where the cluster sits, from points of the reference picture.
//! - [`dial`]: the tachometer, a quarter-ellipse arc with the rpm numbered on it. The accent colour past the red line.
//! - [`arc`]: the arc's path, which the dial draws along.
//! - [`readout`]: the speed, the unit with the H of MPH in gold, and the gear.
//! - [`pursuit`]: the pursuit timer, top right.
//! - [`bar`]: the gauge bars under the dial: N2O, the speed breaker and the boost.
//! - [`map`]: the full-screen world map (the `WorldMapChrome` slot), read from the map view cell.
//! - [`menu`]: the vertical menus (main menu, option categories, pause menu, safe house), read from the menu list cell.
//! - [`skin`]: the colours and fonts the components read, by key.
//!
//! State comes from the same drive feed the Custom theme reads (`custom::DriveFeedCell`), filled once per frame by the HUD.

mod arc;
mod bar;
mod dial;
mod group;
mod layout;
mod map;
pub mod menu;
mod minimap;
mod pursuit;
mod readout;
mod skin;
#[cfg(test)]
mod tests;

pub use bar::{Bar, Gauge};
pub use dial::Dial;
pub use map::{MapSnapshot, WorldMapView, WorldMapViewCell};
pub use menu::VerticalMenu;
pub use minimap::Minimap;
pub use pursuit::PursuitTimer;
pub use readout::Readout;
pub use skin::{palette_skin, skin};

use blackbox_ui::{BuildCx, Component, Empty, SlotId, Theme, ThemeId};

use crate::ui::theme::custom::DriveFeedCell;
use crate::ui::theme::original::FrameCell;
use crate::ui::theme::{ElementId, MenuListCell};

/// The Remastered theme. It holds the drive feed the cluster reads and the legend the themed map draws.
#[derive(Clone, Debug, Default)]
pub struct RemasteredTheme {
    frame: FrameCell,
    drive: DriveFeedCell,
    menus: MenuListCell,
    map: WorldMapViewCell,
}

impl RemasteredTheme {
    /// The Remastered theme whose cluster reads `drive`, whose menus read `menus` and whose map reads `map`. The minimap
    /// reads the game's own map pieces from `frame`.
    pub fn new(frame: FrameCell, drive: DriveFeedCell, menus: MenuListCell, map: WorldMapViewCell) -> Self {
        Self { frame, drive, menus, map }
    }
}

impl Theme for RemasteredTheme {
    fn id(&self) -> ThemeId {
        ThemeId::Remastered
    }

    fn element(&self, slot: SlotId, _cx: &BuildCx<'_>) -> Option<Box<dyn Component>> {
        match ElementId::from_slot(slot)? {
            ElementId::Tachometer => Some(Box::new(Dial::new(self.drive.clone()))),
            ElementId::Speedometer => Some(Box::new(Readout::new(self.drive.clone()))),
            ElementId::Minimap => Some(Box::new(Minimap::new(self.frame.clone(), self.drive.clone()))),
            ElementId::PursuitMeter => Some(Box::new(PursuitTimer::new(self.drive.clone()))),
            ElementId::Nitrous => Some(Box::new(Bar::new(self.drive.clone(), Gauge::Nitrous))),
            ElementId::Speedbreaker => Some(Box::new(Bar::new(self.drive.clone(), Gauge::Speedbreaker))),
            ElementId::Turbo => Some(Box::new(Bar::new(self.drive.clone(), Gauge::Supercharger))),
            // The cluster draws the gear inside the dial, so the original gear digit stays hidden.
            ElementId::GearIndicator => Some(Box::new(Empty)),
            // The icon menus are drawn as vertical lists from the entries the frontend publishes.
            menu @ (ElementId::MainMenu
            | ElementId::Categories
            | ElementId::PauseMenu
            | ElementId::SafeHouseMenu
            | ElementId::OptionTable
            | ElementId::Dialog
            | ElementId::ControlsList
            | ElementId::Jukebox
            | ElementId::Credits) => Some(Box::new(VerticalMenu::new(menu.slot(), self.menus.clone()))),
            // The whole map is drawn here, over the FEng screen the frontend leaves unpresented.
            ElementId::WorldMapChrome => Some(Box::new(WorldMapView::new(self.map.clone()))),
            _ => None,
        }
    }
}
