//! Ids the screens use: messages the packages send to the game, scripts, and the hashes of object names and
//! language labels (docs/specs/frontend-menus.md). Message, script and object ids are `fe_hash_upper` of the
//! name; labels are the language table's own keys.

/// File names of the screens the front end moves between.
pub mod screen {
    pub const SPLASH: &str = "MW_LS_Splash.fng";
    /// The title screen for windows wider than 4:3.
    pub const SPLASH_WIDE: &str = "WS_MW_LS_Splash.fng";
    pub const MAIN_MENU: &str = "MainMenu.fng";
    pub const MAIN_MENU_SUB: &str = "MainMenu_Sub.fng";
    pub const OPTIONS: &str = "Options.fng";
    pub const PAUSE_MENU: &str = "Pause_Main.fng";
    pub const PAUSE_OPTIONS: &str = "Pause_Options.fng";
}

// Messages.
pub const LEAVE_SCREEN: u32 = 0x587C_018B;
pub const EXIT_COMPLETE: u32 = 0xE1FD_E1D1;
pub const INIT_COMPLETE: u32 = 0x35F8_620B;
pub const END_PAD_LEFT: u32 = 0xD711_8934;
pub const END_PAD_RIGHT: u32 = 0xB9B1_7747;
/// The `Quit` button of the main menu answers a mouse click with this message.
pub const QUIT_CLICKED: u32 = 0x6279_9A4C;
pub const MOUSE_LEFT_RELEASED: u32 = 0x7EAB_CA56;
/// The package answers it by switching its buttons back on (a press switches them off while the screen leaves).
pub const INPUT_ENABLE: u32 = 0x8CB8_1F09;

// Scripts.
pub const SCRIPT_HIGHLIGHT: u32 = 0x249D_B7B7;
pub const SCRIPT_UNHIGHLIGHT: u32 = 0x7AB5_521A;
pub const SCRIPT_FORWARD: u32 = 0xDE6E_FF34;

// Objects.
pub const NAME_GROUP: u32 = 0xFB80_CDAC;
pub const GAME_STATS_GROUP: u32 = 0x8913_E195;
/// The group that holds the header string of the pause menu.
pub const PAUSE_HEADER: u32 = 0x8634_04B5;
/// The object whose scripts time the leave animation (its name is only known as a hash).
pub const EVENT_HANDLER: u32 = 0x47FF_4E7C;
pub const OPTION_MASTER: u32 = 0xB046_69E3;
pub const ICON_TITLE: u32 = 0x5E7B_09C9;
pub const ICON_TITLE_SHADOW: u32 = 0x0DFB_7A2E;
pub const TITLE_GROUP: u32 = 0xB71B_576D;
pub const HEADER_TEXT: u32 = 0x42AD_B44C;
/// The `Quit` group of the main menu.
pub const QUIT_BUTTON: u32 = 0xC0A3_2823;

// Textures.
/// The empty icon that pads the ends of an icon menu.
pub const END_OF_SCROLLER: u32 = 0x43B6_310F;

// Language labels.
pub const LABEL_ON: u32 = 0x417B_2604;
pub const LABEL_OFF: u32 = 0x70DF_E5C2;

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_feng::fe_hash_upper;

    #[test]
    fn names_hash_to_their_ids() {
        for (id, name) in [
            (LEAVE_SCREEN, "LEAVE_SCREEN"),
            (EXIT_COMPLETE, "EXIT_COMPLETE"),
            (INIT_COMPLETE, "INIT_COMPLETE"),
            (END_PAD_LEFT, "END_PAD_LEFT"),
            (END_PAD_RIGHT, "END_PAD_RIGHT"),
            (MOUSE_LEFT_RELEASED, "MOUSE_LEFT_RELEASED"),
            (SCRIPT_HIGHLIGHT, "HIGHLIGHT"),
            (SCRIPT_UNHIGHLIGHT, "UNHIGHLIGHT"),
            (SCRIPT_FORWARD, "FORWARD"),
            (ICON_TITLE, "ICON_TITLE"),
            (TITLE_GROUP, "TITLE_GROUP"),
        ] {
            assert_eq!(id, fe_hash_upper(name), "{name}");
        }
    }
}
