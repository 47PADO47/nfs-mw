//! Localized footer captions.
use super::CLEAR_ALL;
use crate::i18n::{self, t};

/// What a footer hint says it does. Most are the game's own hint words; the rest are the catalogue's (`prompts-*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintLabel {
    Accept,
    Back,
    Quit,
    Defaults,
    ViewTutorial,
    Showcase,
    Zoom,
    ViewCar,
    Continue,
    GameStats,
    Select,
    Device,
    Column,
    Remap,
    ChangeMode,
    Move,
    Grab,
    Preview,
    Order,
    ViewRival,
    Randomize,
    Adjust,
    Done,
    // The garage's own hints.
    AddNewCar,
    CustomizeCar,
    DeleteCar,
    Backroom,
    MaxPerformance,
    AddToCart,
    ViewCart,
    RemoveNumbers,
    Remove,
    Add,
    Checkout,
    ClearAll,
    Cancel,
    Help,
    Hide,
}

impl HintLabel {
    /// The label's text in the current language.
    pub fn text(self) -> String {
        let label = match self {
            Self::Accept => 0x2B65_A216,         // H_ACCEPT
            Self::Back => 0x61D5_96D7,           // H_BACK
            Self::Quit => 0xC51F_BF74,           // PC_QUIT
            Self::Defaults => 0x00CA_369E,       // H_DEFAULTS
            Self::ViewTutorial => 0xCE39_D54D,   // VIEW_TUTORIAL
            Self::Showcase => 0xD616_E103,       // H_SHOWCASE
            Self::Zoom => 0xC524_9616,           // PC_ZOOM
            Self::ViewCar => 0x5DE7_9F0B,        // SHOWCASE_VIEW_CAR
            Self::Continue => 0x8098_A54C,       // COMMON_CONTINUE
            Self::GameStats => 0x7F73_40AF,      // COMMON_GAME_STATS
            Self::Select => 0x3FC3_2947,         // COMMON_SELECT
            Self::Randomize => 0x363D_108F,      // H_RANDOMIZE
            Self::Done => 0x8CD6_BD6D,           // COMMON_DONE
            Self::AddNewCar => 0xC259_8BD8,      // MYCAR_ADD_CAR
            Self::CustomizeCar => 0xC984_7935,   // MYCAR_EDIT_CAR
            Self::DeleteCar => 0xD5A1_1802,      // MYCAR_DELETE_CAR
            Self::Backroom => 0xEE11_7BF4,       // H_BACKROOM
            Self::MaxPerformance => 0x972C_3B80, // CM_MAX_PERF
            Self::AddToCart => 0xCFDF_1D5A,      // H_ADD_TO_CART
            Self::ViewCart => 0xE2DF_460A,       // H_VIEW_CART
            Self::RemoveNumbers => 0xBE43_4A38,  // CN_REMOVE_NUMBERS
            Self::Remove => 0x5DAB_CBC0,         // UNCHECK
            Self::Add => 0x028F_EADD,            // CHECK
            Self::Checkout => 0x2557_DE51,       // CSHOP_CHECKOUT
            Self::ClearAll => CLEAR_ALL,         // CSHOP_CLEAR
            Self::Cancel => 0x1A29_4DAD,         // COMMON_CANCEL
            Self::Hide => 0x61D9_0340,
            Self::Help => 0x61D8_F34F, // H_HELP
            Self::Device => return t!("prompts-device"),
            Self::Column => return t!("prompts-column"),
            Self::Remap => return t!("prompts-remap"),
            Self::ChangeMode => return t!("prompts-change-mode"),
            Self::Move => return t!("prompts-move"),
            Self::Grab => 0x84F9_A1E5,
            Self::Preview => 0x325E_1F8F,
            Self::Order => return t!("prompts-order"),
            Self::ViewRival => return t!("prompts-view-rival"),
            Self::Adjust => return t!("prompts-adjust"),
        };
        i18n::game_or_error(label)
    }
}
