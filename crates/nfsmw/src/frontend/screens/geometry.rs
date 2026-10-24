//! Whose places the pointer is tested against on a list screen: the Remastered menus' (`ui/theme/remastered/menu`) or
//! the Custom menus' (`ui/theme/custom/menu`). The two looks lay their lists out differently, so a click lands on what
//! is drawn only when it is tested against the look that drew it. The plugin sets the geometry each frame from the
//! settings, next to the window's aspect.

use glam::Vec2;

use super::Screens;
use crate::frontend::logic::Way;
use crate::input::Action;
use crate::settings::{Settings, UiStyle};
use crate::ui::theme::custom::menu::layout::{self as custom_layout, MenuFrame, Side as CustomSide};
use crate::ui::theme::custom::menu::{dialog_row_at, hint_at as custom_hint_at};
use crate::ui::theme::remastered::menu::layout as remastered;
use crate::ui::theme::{MenuHint, MenuList, MenuStyle};

/// The look whose places a list screen's pointer is tested against.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ListGeometry {
    /// The Remastered vertical menus.
    #[default]
    Remastered,
    /// The Custom menus, at the `[custom_hud]` text size.
    Custom { text_size: f32 },
}

impl ListGeometry {
    /// The geometry of the style the settings chose. Original draws no lists, so it keeps the Remastered geometry,
    /// which no screen asks for then.
    pub fn of(settings: &Settings) -> Self {
        match settings.ui_style {
            UiStyle::Custom => Self::Custom { text_size: settings.hud_options.text_size },
            UiStyle::Original | UiStyle::Remastered => Self::Remastered,
        }
    }

    /// The entry of `list` under a point, on a window `aspect` wide.
    pub fn row_at(self, point: Vec2, list: &MenuList, aspect: f32) -> Option<usize> {
        let rest = list.entries.len().saturating_sub(list.first);
        let text_size = match self {
            Self::Custom { text_size } if list.style != MenuStyle::Blacklist => text_size,
            _ => return remastered_row_at(point, list, aspect),
        };
        if list.style == MenuStyle::Dialog {
            return dialog_row_at(point, list, aspect, text_size);
        }
        let frame = MenuFrame::new(aspect, text_size);
        frame.row_at(point, custom_layout::shown(rest), list.has_values()).map(|row| list.first + row)
    }

    /// Which way a click at `point` presses a settings row: back on the left chevron, forward on the right.
    pub fn way_at(self, point: Vec2, aspect: f32) -> Way {
        let Self::Custom { text_size } = self else {
            return match remastered::side_at(point.x - remastered::left_edge(aspect)) {
                remastered::Side::Left => Way::Back,
                remastered::Side::Right => Way::Forward,
                remastered::Side::Middle => Way::Stay,
            };
        };
        match MenuFrame::new(aspect, text_size).side_at(point.x) {
            CustomSide::Left => Way::Back,
            CustomSide::Right => Way::Forward,
            CustomSide::Middle => Way::Stay,
        }
    }

    /// How far along the value column of a row a point is, from 0 at its left edge to 1 at its right.
    pub fn along_at(self, point: Vec2, aspect: f32) -> f32 {
        let Self::Custom { text_size } = self else {
            return remastered::along_value(point.x - remastered::left_edge(aspect));
        };
        MenuFrame::new(aspect, text_size).along_value(point.x)
    }

    /// The footer hint under a point whose click presses an action.
    pub fn hint_at(self, hints: &[MenuHint], point: Vec2, aspect: f32) -> Option<Action> {
        let Self::Custom { text_size } = self else {
            return remastered::hint_at(hints, point, aspect);
        };
        custom_hint_at(hints, point, aspect, text_size)
    }
}

/// The entry of `list` under a point in the Remastered places.
fn remastered_row_at(point: Vec2, list: &MenuList, aspect: f32) -> Option<usize> {
    if list.style == MenuStyle::Dialog {
        return remastered::dialog_row_at(point, list);
    }
    if list.style == MenuStyle::Blacklist {
        return remastered::blacklist_row_at(point, list);
    }
    let shown = remastered::shown(list.entries.len().saturating_sub(list.first));
    remastered::row_at(point, shown, aspect, list.has_values()).map(|row| list.first + row)
}

impl Screens {
    /// Sets the look whose places the pointer is tested against (`ListGeometry::of`), once per frame.
    pub fn set_list_geometry(&mut self, geometry: ListGeometry) {
        self.geometry = geometry;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Partial;
    use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry};

    fn list(style: MenuStyle) -> MenuList {
        MenuList {
            title: "Options".into(),
            message: String::new(),
            entries: (0..4).map(|i| MenuEntry::plain(format!("Row {i}"), true)).collect(),
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        }
    }

    #[test]
    fn the_style_chooses_the_geometry_with_the_text_size() {
        let mut settings = Settings::from(Partial::default());
        settings.ui_style = UiStyle::Remastered;
        assert_eq!(ListGeometry::of(&settings), ListGeometry::Remastered);
        settings.ui_style = UiStyle::Custom;
        settings.hud_options.text_size = 1.2;
        assert_eq!(ListGeometry::of(&settings), ListGeometry::Custom { text_size: 1.2 });
    }

    #[test]
    fn a_click_lands_on_the_row_each_look_draws() {
        let custom = ListGeometry::Custom { text_size: 1.0 };
        let frame = MenuFrame::new(DEFAULT_ASPECT, 1.0);
        let on_custom = Vec2::new(frame.text_left(), frame.row_y(2));
        assert_eq!(custom.row_at(on_custom, &list(MenuStyle::Sub), DEFAULT_ASPECT), Some(2));
        let on_remastered = Vec2::new(remastered::left_edge(DEFAULT_ASPECT) + 100.0, remastered::row_y(1));
        assert_eq!(ListGeometry::Remastered.row_at(on_remastered, &list(MenuStyle::Sub), DEFAULT_ASPECT), Some(1));
        let scrolled = MenuList { first: 1, ..list(MenuStyle::Sub) };
        assert_eq!(custom.row_at(on_custom, &scrolled, DEFAULT_ASPECT), Some(3), "the window's first entry counts");
    }

    #[test]
    fn a_click_on_a_custom_chevron_steps_that_way() {
        let custom = ListGeometry::Custom { text_size: 1.0 };
        let column = MenuFrame::new(DEFAULT_ASPECT, 1.0).value_column(0.0);
        let at = |x: f32| custom.way_at(Vec2::new(x, 0.0), DEFAULT_ASPECT);
        assert_eq!(at(column.min_x() + 2.0), Way::Back);
        assert_eq!(at(column.max_x() - 2.0), Way::Forward);
        assert_eq!(at(column.x), Way::Stay);
        assert!((custom.along_at(Vec2::new(column.x, 0.0), DEFAULT_ASPECT) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn the_custom_list_window_matches_the_frontends() {
        assert_eq!(custom_layout::MAX_ROWS, remastered::MAX_ROWS, "both looks show the entries the window holds");
    }
}
