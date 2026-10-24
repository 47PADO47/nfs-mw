//! Which look draws the menus, as the screens ask it: whether a theme lays the menus out as lists from the entries the
//! screens publish ([`Cx::list_look`]), and whether that theme is Remastered ([`Cx::remastered_look`]).
//!
//! Two looks draw lists: Remastered and Custom (`ui/theme/remastered/menu`, `ui/theme/custom/menu`). Custom draws the
//! main menu, the option categories, the pause menu, the safe house, the option pages and the dialogs. The other list
//! screens (the key list, the jukebox, the credits, the Blacklist and the rap sheet) are drawn as lists by Remastered
//! only, so they ask [`Cx::remastered_look`] and keep their FEng packages under Custom.

use super::Cx;
use crate::settings::UiStyle;

/// Whether `style` draws the menus as lists, when no mod theme took over (`mod_theme` false).
pub fn list_look_of(style: UiStyle, mod_theme: bool) -> bool {
    if mod_theme {
        return false;
    }
    matches!(style, UiStyle::Remastered | UiStyle::Custom)
}

/// Whether the Remastered look draws the menus, when no mod theme took over (`mod_theme` false).
pub fn remastered_look_of(style: UiStyle, mod_theme: bool) -> bool {
    !mod_theme && style == UiStyle::Remastered
}

impl Cx<'_> {
    /// Whether a mod theme took the menus over (the Mods page's theme choice).
    fn mod_theme(&self) -> bool {
        self.mods.is_some_and(|mods| mods.ui_theme_choice().is_some())
    }

    /// Whether a list look (Remastered or Custom) draws the menus: the screen then publishes its entries as a list
    /// (`ScreenLogic::list`) and is driven as one, and the theme draws it.
    pub fn list_look(&self) -> bool {
        list_look_of(self.settings.ui_style, self.mod_theme())
    }

    /// Whether the Remastered look draws the menus. The list screens Custom does not draw yet ask this rather than
    /// [`Cx::list_look`], so under Custom they keep their own packages.
    pub fn remastered_look(&self) -> bool {
        remastered_look_of(self.settings.ui_style, self.mod_theme())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_looks_are_remastered_and_custom_unless_a_mod_theme_took_over() {
        let cases = [
            (UiStyle::Original, false, false, false),
            (UiStyle::Remastered, false, true, true),
            (UiStyle::Custom, false, true, false),
            (UiStyle::Original, true, false, false),
            (UiStyle::Remastered, true, false, false),
            (UiStyle::Custom, true, false, false),
        ];
        for (style, mod_theme, list, remastered) in cases {
            assert_eq!(list_look_of(style, mod_theme), list, "{style:?}, mod theme {mod_theme}: list look");
            assert_eq!(remastered_look_of(style, mod_theme), remastered, "{style:?}, mod theme {mod_theme}");
        }
    }

    #[test]
    fn every_style_is_covered() {
        for style in UiStyle::ALL {
            let expected = style != UiStyle::Original;
            assert_eq!(list_look_of(style, false), expected, "{style:?}");
        }
    }
}
