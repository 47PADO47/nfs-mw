//! The car select opened by the LAN lobby (MP-30): the package's leave plays as usual, then the pick (Accept or the
//! Showcase) or Back goes to the LAN menu (`frontend/lan/link.rs`) and the main menu comes back under it. No transmission is
//! asked: a session car drives with the transmission setting.

use super::{CarSelect, Leave};
use crate::frontend::ids::screen;
use crate::frontend::lan::link;
use crate::frontend::logic::{Args, Command, Cx};

impl CarSelect {
    /// The package has left: answer the LAN menu.
    pub(super) fn finish_lan(&mut self, cx: &mut Cx, way: Option<Leave>) {
        let Some(way) = way else { return };
        let pick = match way {
            Leave::Back => None,
            Leave::Accept | Leave::Showcase => self.entry().map(|entry| match &entry.preset {
                Some(preset) => format!("{}:{preset}", entry.car.folder),
                None => entry.car.folder.clone(),
            }),
        };
        link::car_answer(pick);
        cx.send(Command::Switch(screen::MAIN_MENU.into(), Args::default()));
    }
}
