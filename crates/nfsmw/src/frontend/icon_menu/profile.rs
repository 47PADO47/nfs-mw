//! The loaded profile on the icon menus (docs/specs/career-parity-menus.md, section 2): its name in the box at the bottom
//! left of the main menu, the career menu and the alias manager, and on the main menu the statistics card that the Game
//! Stats button (key 1) shows and hides.

use super::{Cx, IconMenu, Kind, ids};
use crate::career::CareerHandle;

/// The string inside the name box, and the group around the box in `MainMenu_Sub.fng` (hidden until the game shows it).
const NAME: u32 = 0xEB40_6FEC;
const NAME_BOX_PARENT: u32 = 0xC874_22F7;
/// The script that shows `NAME_BOX_PARENT`.
const SHOW: u32 = 0x001C_A7C0;
/// The card's values (`GAME_COMPLETED_DATA`, `CAREER_COMPLETED_DATA`, `RAP_SHEET_ITEMS_DATA_2`, `CHALLENGE_SERIES_DATA_1`,
/// `CHALLENGE_SERIES_DATA_2`).
const GAME: u32 = 0xAAEC_91EE;
const CAREER: u32 = 0x6A5E_95C6;
const RAP: u32 = 0x4AA3_2965;
/// `RAP_SHEET_ITEMS_DATA_1`: the original hides it and gives it no value.
const RAP_ITEMS: u32 = 0x4AA3_2964;
const SERIES_COUNT: u32 = 0xD3C8_4175;
const SERIES_PERCENT: u32 = 0xD3C8_4176;
/// The package's messages that show and hide the card, and the one the Game Stats button sends the game (key 1).
const STATS_APPEAR: u32 = 0x2474_A660;
const STATS_LEAVE: u32 = 0xDB11_E414;
pub(super) const GAME_STATS_BUTTON: u32 = 0xC519_BFC4;

impl IconMenu {
    /// Shows the loaded profile's name, and on the main menu its card; with no profile loaded the main menu hides both
    /// groups, as the original does (`profile + 3` clear).
    pub(super) fn show_profile(&mut self, cx: &mut Cx) {
        let profile = CareerHandle::shared().with_loaded(|career| (career.alias.clone(), career.card()));
        match (self.kind, profile) {
            (Kind::Main, None) => {
                cx.hide(ids::NAME_GROUP, true);
                cx.hide(ids::GAME_STATS_GROUP, true);
            }
            (Kind::Main, Some((alias, card))) => {
                cx.hide(ids::NAME_GROUP, false);
                cx.hide(ids::GAME_STATS_GROUP, false);
                cx.text_object(NAME, &alias);
                cx.text_object(GAME, &percent(card.game));
                cx.text_object(CAREER, &percent(card.career));
                cx.text_object(RAP, &percent(card.rap));
                cx.hide(RAP_ITEMS, true);
                cx.text_object(SERIES_COUNT, &format!("{}/{}", card.series_done, card.series_total));
                cx.text_object(SERIES_PERCENT, &percent(card.series));
            }
            (Kind::Career, Some((alias, _))) => {
                cx.script(NAME_BOX_PARENT, SHOW);
                cx.text_object(NAME, &alias);
            }
            (Kind::AliasManager, Some((alias, _))) => cx.text_object(NAME, &alias),
            (Kind::AliasManager, None) => cx.hide(ids::NAME_GROUP, true),
            _ => {}
        }
    }

    /// The Game Stats button: the card appears, or leaves when it is up. Only the main menu with a profile has one.
    pub(super) fn toggle_stats(&mut self, cx: &mut Cx) {
        if self.kind != Kind::Main || !CareerHandle::shared().is_loaded() {
            return;
        }
        self.stats_open = !self.stats_open;
        let message = if self.stats_open { STATS_APPEAR } else { STATS_LEAVE };
        cx.rt.post_to_package(cx.package, message);
    }
}

/// A percentage as the card prints it, as the language writes one.
fn percent(value: u8) -> String {
    crate::i18n::fmt::percent(i64::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_percentage_has_its_sign() {
        assert_eq!(percent(0), "0%");
        assert_eq!(percent(100), "100%");
    }
}
