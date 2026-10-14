//! The icons of each icon menu: what an icon does, its picture and its name (docs/specs/frontend-menus.md, section 2).
//! `icon_menu.rs` binds them to the package's objects and scrolls them.

use super::icon_menu::Kind;
use super::logic::Category;

/// What an icon does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Id {
    Career,
    Challenge,
    QuickRace,
    MyCars,
    /// The online and LAN entries have no screen here yet, so they are greyed out in the main menu.
    Online,
    LanPlay,
    AliasManager,
    Options,
    Player,
    /// The key list (`controls_screen.rs`).
    Controls,
    /// The sensitivity and camera rows (`widget_menu.rs`).
    ControlOptions,
    EaTrax,
    Credits,
    Category(Category),
    Resume,
    QuitToMenu,
}

#[derive(Clone, Copy, Debug)]
pub struct Icon {
    pub id: Id,
    /// The texture key of the icon, and the language labels of its name. Texture 0 draws no picture.
    pub texture: u32,
    pub name: u32,
    pub enabled: bool,
}

const fn icon(id: Id, texture: u32, name: u32, enabled: bool) -> Icon {
    Icon { id, texture, name, enabled }
}

/// The icons of a menu: `(texture, name label)` as the original screens give them.
pub fn icons(kind: Kind) -> Vec<Icon> {
    match kind {
        // Career and Quick Race both start free roam here. Challenge Series, My Cars, Online, LAN and Alias Manager
        // (the original's profile manager, which it adds only when the memory card is enabled) are greyed out.
        // Online and LAN are not in this build's original main menu; the user asked for them, so they are listed
        // without a screen (docs/specs/frontend-menus.md, 2.1). The main menu has eleven objects, so the row
        // windows over them (`icon_window.rs`).
        Kind::Main => vec![
            icon(Id::Career, 0x0370_4F3D, 0x5815_A2B5, true),
            icon(Id::Challenge, 0x9A96_2438, 0xCC8C_B746, false),
            icon(Id::QuickRace, 0x4E6F_BB02, 0x5402_0A7A, true),
            icon(Id::MyCars, 0xB0C4_6023, 0x1AFD_5BE6, false),
            // The texture of the option list's Online entry; the label is the one it shows ("Online Options").
            icon(Id::Online, 0x5A5E_5137, 0x7D34_7E05, false),
            // The main menu's LAN label. No picture for it is recorded, so the icon is blank.
            icon(Id::LanPlay, 0, 0x03F6_F2BF, false),
            icon(Id::AliasManager, 0x6B30_3856, 0xBCB1_8F38, false),
            icon(Id::Options, 0x3058_FE37, 0x19A8_C0AF, true),
        ],
        // The original's option list (docs/specs/frontend-menus.md, 2.2): Player, EA Trax and Credits have no screen
        // here yet, so they are greyed out; EA Trax and Credits are left out in the pause menu. Online is left out too:
        // the original shows it only in online mode, which this build never is.
        // Controls opens the key list; Control Options (the sensitivity and camera rows) sits beside it.
        Kind::Categories { pause } => {
            let mut list = vec![
                icon(Id::Category(Category::Audio), 0xF37A_F144, 0xE76C_D783, true),
                icon(Id::Category(Category::Video), 0x8A00_6328, 0xE8E2_4508, true),
                icon(Id::Category(Category::Gameplay), 0x4DF9_8FB2, 0xD0CF_6EE1, true),
                icon(Id::Player, 0xD708_EFEF, 0xF760_EABE, false),
                icon(Id::Controls, 0xA04A_7B26, 0x04DC_6DB5, true),
                icon(Id::ControlOptions, crate::ui::input_icons::ATLAS, 0, true),
            ];
            if !pause {
                list.push(icon(Id::EaTrax, 0, 0xDCFB_6B36, false));
                list.push(icon(Id::Credits, 0, 0x0905_101F, false));
            }
            list
        }
        Kind::Pause => vec![
            icon(Id::Resume, 0x12BB_5EA2, 0x01BD_185C, true),
            icon(Id::Options, 0x520D_E4E3, 0x2B5A_03A8, true),
            icon(Id::QuitToMenu, 0x4C9E_34E6, 0xE950_B7AF, true),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menus_have_the_original_icons() {
        let main = icons(Kind::Main);
        assert_eq!(main.len(), 8, "the original's main menu, Alias Manager included, plus LAN and Online");
        assert!(main[0].enabled && !main[1].enabled, "career works, the challenge series do not exist yet");
        assert!(
            main.iter()
                .filter(|i| i.id != Id::Career && i.id != Id::QuickRace && i.id != Id::Options)
                .all(|i| !i.enabled)
        );
        let categories = icons(Kind::Categories { pause: false });
        assert_eq!(
            categories.len(),
            8,
            "the offline option list plus Control Options, which fits the 12 package slots"
        );
        let controls = categories.iter().find(|i| i.id == Id::Controls).expect("the Controls icon");
        assert_eq!(controls.texture, 0xA04A_7B26, "OMController's picture (docs/specs/frontend-menus.md, 2.2)");
        let options = categories.iter().position(|i| i.id == Id::ControlOptions).expect("Control Options");
        assert_eq!(categories[options - 1].id, Id::Controls, "Control Options follows Controls");
        let player = categories.iter().find(|i| i.id == Id::Player).expect("the Player category");
        assert_eq!((player.texture, player.name), (0xD708_EFEF, 0xF760_EABE), "the original's Player icon");
        assert_eq!(icons(Kind::Categories { pause: true }).len(), 6, "no EA Trax or Credits in the pause menu");
        assert_eq!(icons(Kind::Pause)[0].name, 0x01BD_185C, "Resume Free Roam");
    }

    #[test]
    fn lan_and_online_are_listed_dimmed() {
        let main = icons(Kind::Main);
        let online = main.iter().find(|i| i.id == Id::Online).expect("Online");
        assert_eq!((online.texture, online.name), (0x5A5E_5137, 0x7D34_7E05));
        assert!(!online.enabled);
        let lan = main.iter().find(|i| i.id == Id::LanPlay).expect("LAN");
        assert_eq!(lan.name, 0x03F6_F2BF, "the LAN main-menu label");
        assert!(!lan.enabled);
    }
}
