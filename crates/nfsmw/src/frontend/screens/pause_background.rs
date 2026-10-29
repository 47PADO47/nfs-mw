//! Pause menus share the install's separately authored background package beneath the interactive stack.

use super::{Screen, Screens, shown};
use crate::frontend::logic::Plain;

pub(super) const NAME: &str = "InGameBackground.fng";

impl Screens {
    pub(super) fn open_pause_background(&mut self, name: &str) {
        if !name.to_ascii_lowercase().starts_with("pause_") || self.stack.iter().any(is_background) {
            return;
        }
        let Some(package) = self.catalog.find(NAME).cloned() else { return };
        let id = self.runtime.load(package);
        self.stack.insert(
            0,
            Screen {
                id,
                name: NAME.into(),
                logic: Box::new(Plain),
                slot: None,
                look: None,
                pose: None,
                shown: shown::Shown::default(),
            },
        );
    }

    pub(super) fn visible_screens(&self) -> impl Iterator<Item = &Screen> {
        let start = self.visible_start();
        self.stack
            .iter()
            .enumerate()
            .filter(move |(index, screen)| *index >= start || is_background(screen))
            .map(|(_, screen)| screen)
    }
}

fn is_background(screen: &Screen) -> bool {
    screen.name.eq_ignore_ascii_case(NAME)
}
