//! The picture of a menu entry, beside its label. The menus that have pictures (the main menu, the option categories,
//! the pause menu, the safe house and the career and race menus, whose entries carry the game's own icon textures) show
//! them whatever the HUD theme is: the picture is part of the screen, not a palette's choice. A list in which some
//! entry has a picture gives every entry the room for one, and an entry with none draws the pictogram of the HUD's icon
//! set (`notify/icon.rs`) in its place, so the labels stay in one column and no row is left bare.

use blackbox_ui::{Component, DEFAULT_Z, Image, Rect};

use super::Parts;
use crate::ui::theme::custom::notify::icon::icon;
use crate::ui::theme::custom::tokens::{fade, keys};
use crate::ui::theme::{MenuEntry, MenuList};

/// A picture's side and the gap after it, at scale 1.
pub(super) const SIZE: f32 = 22.0;
const GAP: f32 = 9.0;
/// A disabled entry's picture alpha.
const DISABLED_ALPHA: u8 = 95;
/// The catalogue id of the pictogram an entry with no picture draws (an id with no pictogram of its own is the ringed
/// dot).
const FALLBACK: &str = "entry";

/// Whether `list` shows pictures: any of its entries has one.
pub(super) fn shows(list: &MenuList) -> bool {
    list.entries.iter().any(|entry| entry.icon.is_some())
}

/// How far a list's labels move right to make room for a picture, at `scale`; nothing for a list without pictures.
pub(super) fn shift(list: &MenuList, scale: f32) -> f32 {
    match shows(list) {
        true => (SIZE + GAP) * scale,
        false => 0.0,
    }
}

impl Parts<'_, '_> {
    /// The picture of `entry` on the row at `y`: its texture, or the fallback pictogram when it has none.
    pub(super) fn picture(&mut self, entry: &MenuEntry, y: f32) {
        let side = SIZE * self.frame().scale();
        let rect = Rect::new(self.frame().text_left() + side / 2.0, y, side, side);
        let Some(icon_texture) = entry.icon else {
            self.pictogram(entry, rect);
            return;
        };
        let alpha = match entry.enabled {
            true => 255,
            false => DISABLED_ALPHA,
        };
        let mut image = Image::raw(icon_texture.texture).with_uv(icon_texture.uv).with_tint([255, 255, 255, alpha]);
        image.layout(self.cx(), rect);
        self.add(image);
    }

    /// The fallback pictogram in the secondary text colour (at half alpha when disabled).
    fn pictogram(&mut self, entry: &MenuEntry, rect: Rect) {
        let Some(colour) = self.cx().skin.colour(keys::TEXT_SECONDARY) else { return };
        let opacity = match entry.enabled {
            true => 1.0,
            false => super::list::DISABLED,
        };
        self.add(icon(FALLBACK, shrunk(rect), fade(colour, opacity), DEFAULT_Z));
    }
}

/// `rect` inside its own margin, so a pictogram sits smaller than the textured pictures.
fn shrunk(rect: Rect) -> Rect {
    Rect::new(rect.x, rect.y, rect.w * 0.7, rect.h * 0.7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme::custom::menu::tests::{built, list, text_at};
    use crate::ui::theme::{ElementId, MenuIcon, MenuStyle};
    use blackbox_ui::{NodeKind, UiNode};

    fn with_picture(label: &str, texture: u32) -> MenuEntry {
        MenuEntry { icon: Some(MenuIcon { texture, uv: [0.0, 0.0, 1.0, 1.0] }), ..MenuEntry::plain(label, true) }
    }

    fn textured(nodes: &[UiNode], texture: u32) -> usize {
        let is = |node: &&UiNode| matches!(node.kind, NodeKind::Image { texture: t, .. } if t == texture);
        nodes.iter().filter(is).count()
    }

    fn pictured() -> MenuList {
        let mut pictured = list(MenuStyle::Main, 0);
        pictured.entries[0] = with_picture("Career", 4242);
        pictured.entries[2] = with_picture("Quick Race", 4343);
        pictured
    }

    #[test]
    fn a_list_with_pictures_draws_them_and_moves_every_label_over() {
        let plain = built(ElementId::MainMenu, list(MenuStyle::Main, 0));
        let nodes = built(ElementId::MainMenu, pictured());
        assert_eq!((textured(&nodes, 4242), textured(&nodes, 4343)), (1, 1), "each picture is drawn once");
        let x = |nodes: &[UiNode], label: &str| text_at(nodes, label).world.w_axis.x;
        assert!(x(&nodes, "Career") > x(&plain, "Career") + 1.0, "the label makes room for the picture");
        let column = (x(&nodes, "Career") - x(&nodes, "Challenge Series")).abs();
        assert!(column < 0.5, "an entry without a picture keeps the column: {column}");
    }

    #[test]
    fn an_entry_without_a_picture_in_a_pictured_list_draws_the_fallback_pictogram() {
        let with = built(ElementId::MainMenu, pictured()).len();
        let mut all = pictured();
        all.entries[1] = with_picture("Challenge Series", 4444);
        let full = built(ElementId::MainMenu, all).len();
        assert!(with > full, "the bare row's pictogram strokes outnumber a picture's one quad");
        let bare = built(ElementId::MainMenu, list(MenuStyle::Main, 0)).len();
        assert!(with > bare, "a list with pictures draws more than one without");
    }

    #[test]
    fn a_list_without_pictures_makes_no_room() {
        assert!(!shows(&list(MenuStyle::Main, 0)));
        assert_eq!(shift(&list(MenuStyle::Main, 0), 1.0), 0.0);
        assert!(shift(&pictured(), 1.0) > 0.0);
    }
}
