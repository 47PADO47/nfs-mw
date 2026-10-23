//! Which of the world's egui panels draw this frame. The UI pass is skipped when none does (and no developer panel is
//! up either), so each flag here is read from the same helper the panel's drawing code checks.

use super::*;

/// Whether each conditional panel of the world's `ui` has something to draw.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct Panels {
    pub interact_prompt: bool,
    pub mod_message: bool,
    pub pursuit_end: bool,
    pub race_card: bool,
    pub race_message_card: bool,
    pub nis_bars: bool,
    pub subtitles: bool,
    pub nis_screen: bool,
}

impl Panels {
    /// Whether any panel draws.
    pub fn any(self) -> bool {
        let Self {
            interact_prompt,
            mod_message,
            pursuit_end,
            race_card,
            race_message_card,
            nis_bars,
            subtitles,
            nis_screen,
        } = self;
        interact_prompt
            || mod_message
            || pursuit_end
            || race_card
            || race_message_card
            || nis_bars
            || subtitles
            || nis_screen
    }
}

impl WorldScene {
    /// The panels `ui` would draw now.
    pub(super) fn panels(&self) -> Panels {
        Panels {
            interact_prompt: self.entry.popup.visible(),
            mod_message: crate::hud::mod_message_up(),
            pursuit_end: self.pursuit_end().is_some(),
            race_card: self.race_card().is_some(),
            race_message_card: self.race_card_text().is_some(),
            nis_bars: self.nis_bars_up(),
            subtitles: self.subtitles_up(),
            nis_screen: self.nis.screen().is_showing(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Panels;

    #[test]
    fn no_panel_up_draws_nothing() {
        assert!(!Panels::default().any());
    }

    #[test]
    fn any_one_panel_makes_the_scene_want_the_pass() {
        let each = [
            Panels { interact_prompt: true, ..Panels::default() },
            Panels { mod_message: true, ..Panels::default() },
            Panels { pursuit_end: true, ..Panels::default() },
            Panels { race_card: true, ..Panels::default() },
            Panels { race_message_card: true, ..Panels::default() },
            Panels { nis_bars: true, ..Panels::default() },
            Panels { subtitles: true, ..Panels::default() },
            Panels { nis_screen: true, ..Panels::default() },
        ];
        for panels in each {
            assert!(panels.any(), "{panels:?}");
        }
    }
}
