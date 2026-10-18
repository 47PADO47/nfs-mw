//! The rap sheet login pair (`RapSheetLogin.fng`, `RapSheetLogin2.fng`): a boot animation that Accept skips, then the
//! main page. Back returns to the safe house menu. There is no password check (`docs/specs/career-rapsheet.md`
//! section 2). The package runs the animation; this logic only answers its messages.

use blackbox_feng::ids::{PAD_ACCEPT, PAD_BACK};
use nfsmw_career::rapsheet::{LoginPage, Page};
use nfsmw_career::safehouse::FrontEndContext;

use super::open;
use crate::career::CareerHandle;
use crate::frontend::ids::{EXIT_COMPLETE, MOUSE_LEFT_RELEASED};
use crate::frontend::logic::{Cx, ScreenLogic};

/// The list object of `RapSheetLogin2.fng` that shows the profile name as its row (`fe_hash_upper`).
const PROFILE_ROW: u32 = 0x03CC_94D6;

/// The profile's row on the second login page: `> <name>`.
pub fn profile_row(name: &str) -> String {
    format!("> {name}")
}

/// Where the login goes when its leave animation ends. The front end bit of the rap sheet is set while the pages are
/// up; the port does not keep that bit, so the value passed is the one the screen runs under.
pub fn next_page(page: LoginPage) -> Page {
    let mut context = FrontEndContext(FrontEndContext::CAREER.0 | FrontEndContext::RAP_SHEET.0);
    page.exit_complete(&mut context)
}

/// `RapSheetLogin.fng` (`second` false) or `RapSheetLogin2.fng` (`second` true).
pub struct LoginScreen {
    page: LoginPage,
    second: bool,
}

impl LoginScreen {
    /// The first login page.
    pub fn first() -> Self {
        Self { page: LoginPage::login(), second: false }
    }

    /// The second login page, which lists the profile.
    pub fn second() -> Self {
        Self { page: LoginPage::login2(), second: true }
    }

    fn exit(&self, cx: &mut Cx) {
        cx.send(open(next_page(self.page)));
    }
}

impl ScreenLogic for LoginScreen {
    fn start(&mut self, cx: &mut Cx) {
        if !self.second {
            return;
        }
        let alias = CareerHandle::shared().with(|career| career.alias.clone()).unwrap_or_default();
        if let Some(row) = cx.object(PROFILE_ROW) {
            cx.rt.set_text(row, profile_row(&alias));
        }
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            // The UI sound (`0x67` or `0x68`) is not played: the screens have no sound call yet.
            PAD_ACCEPT | MOUSE_LEFT_RELEASED => {
                self.page.accept();
            }
            PAD_BACK => self.page.back(),
            EXIT_COMPLETE => self.exit(cx),
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "login_tests.rs"]
mod tests;
