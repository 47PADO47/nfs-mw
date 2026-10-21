//! The career screens (`docs/specs/career-*.md`): the safe house menu, the Blacklist, the rap sheet, the save
//! screens, the region unlock, the car lot and the SMS inbox. They read and change the career through
//! `CareerHandle::shared()` (the screens are made by name and get no constructor data).

pub mod blacklist;
pub mod car_lot;
pub(crate) mod car_lot_bars;
mod car_lot_first;
pub mod car_select;
mod car_select_view;
pub mod career_menu;
pub mod end_game_page;
pub mod markers;
pub mod rap_sheet;
pub mod region_unlock;
pub mod safehouse_menu;
pub mod save;
pub mod sms;
pub mod sms_popup;

use super::logic::{Args, Cx, ScreenLogic};

/// The logic of the career screen called `name` (lower case), or `None` when `name` is no career screen.
pub fn make(name: &str, args: Args) -> Option<Box<dyn ScreenLogic>> {
    blacklist::make(name, args)
        .or_else(|| car_lot::make(name, args))
        .or_else(|| rap_sheet::make(name, args))
        .or_else(|| region_unlock::make(name, args))
        .or_else(|| markers::make(name, args))
        .or_else(|| save::make(name, args))
        .or_else(|| sms::make(name, args))
        .or_else(|| end_game_page::make(name))
}

/// Posted to `MC_Main.fng` before [`INITIALIZE_SCREEN`]: the memory card prompt's pre-message.
pub const MC_MAIN_PREPARE: u32 = 0xC0F2_AE7C;
/// The init message the memory card screens post once; it starts the alpha fade-in of their content.
pub const INITIALIZE_SCREEN: u32 = 0x71FD_0F10;
/// Posted to the rival screens opened outside the career (context bit 17 clear).
pub const RIVAL_OPENED_PLAIN: u32 = 0xAF92_2178;
/// Posted to the rival screens opened from the career (context bit 17 set): reveals the rival content.
pub const RIVAL_OPENED_CAREER: u32 = 0x0B21_A45F;
/// Posted after the career open message by the rival challenge widget init and the rival list populate.
pub const RIVAL_LIST_READY: u32 = 0xB4C1_44B1;
/// Starts the rival challenge's content scripts: its handler (object `0x104698`) fires the messages the rival rows
/// fade in on. Found in the package's message responses; who posts it in the original is not traced.
pub const RIVAL_CONTENT_START: u32 = 0x5FF4_5174;
/// Reveals the rival bio's content group (`0x116316`), which answers only this message. Found in the package's
/// message responses; who posts it in the original is not traced.
pub const RIVAL_BIO_CONTENT: u32 = 0x8BBA_B3ED;

/// Posts the open-time messages of a career package, in order. The package draws its content only after these
/// (`docs/specs/career-safehouse-2.md` section 11). Call it first in the screen's `start`.
pub fn post_opening(cx: &mut Cx, messages: &[u32]) {
    for &m in messages {
        cx.rt.post_to_package(cx.package, m);
    }
}
