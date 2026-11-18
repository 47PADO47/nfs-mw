//! The race's message card: `SixDaysLater.fng` (one of the screen files), the full-screen package a race's
//! `IntroMessageID` opens (`docs/specs/story-flow-2.md` F-04; the original opens it before `16.1.1`). From install data:
//! a black `BasePoly.tga` panel, a `Number2.tga` image and the string "SIX DAYS LATER..." (label `0x290C9B44`). The
//! package runs its own scripts from its init script: the panel fades in over 300 ticks and holds 4200, and at tick 200
//! of the hold it sends the string the message that runs the string's 200 and 4000 tick scripts. The HUD loads it into
//! its runtime while [`super::HudState::message_card`] is set and draws it alone over the frame; the world's text
//! stand-in is drawn only when the install has no such package ([`message_card_drawn`]).

use std::sync::atomic::{AtomicBool, Ordering};

use blackbox_feng::package::Package;
use blackbox_feng::{PackageId, Runtime};

/// The card's package name.
pub const CARD_PACKAGE: &str = "SixDaysLater.fng";
/// The text of the card's string in the package, for the world's stand-in: the language table has no text for its label
/// `0x290C9B44`, so the package's own text is what shows. From install data.
pub const CARD_TEXT: &str = "SIX DAYS LATER...";
/// The tick at which the last of the card's own scripts ends: the string's two scripts (200 and 4000 ticks) start at
/// tick 500 (the panel's 300-tick fade, then its message at tick 200 of the hold), so they end at tick 4700. From install
/// data (`nfsmw dump-screen SixDaysLater.fng`). What closes the card in the original is native and not known.
pub const CARD_TICKS: f32 = 4700.0;

/// Whether the HUD has the card's package, so it draws the card itself.
static DRAWN: AtomicBool = AtomicBool::new(false);

/// Whether the HUD draws the message card from the install's package; the world draws its text stand-in otherwise.
pub fn message_card_drawn() -> bool {
    DRAWN.load(Ordering::Relaxed)
}

/// The card's package, and its id in the runtime while it is up.
pub struct MessageCard {
    package: Package,
    shown: Option<PackageId>,
}

impl MessageCard {
    /// The card from its package. The HUD draws the card from now on.
    pub fn new(package: Package) -> Self {
        DRAWN.store(true, Ordering::Relaxed);
        Self { package, shown: None }
    }

    /// Loads the card while `shown` is set, so its scripts start from their beginning each time, and unloads it when
    /// that clears.
    pub fn set_shown(&mut self, runtime: &mut Runtime, shown: bool) {
        if shown == self.shown.is_some() {
            return;
        }
        match self.shown.take() {
            Some(id) => runtime.unload(id),
            None => self.shown = Some(runtime.load(self.package.clone())),
        }
    }

    /// The card's package, while it is up.
    pub fn shown(&self) -> Option<PackageId> {
        self.shown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_install::GameDir;

    /// The card's timing against the package: the panel's init chain, and the string's scripts that its message starts.
    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_card_ends_when_the_packages_last_script_ends() {
        let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(std::path::PathBuf::from(dir)).unwrap();
        let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
        let package = catalog.find(CARD_PACKAGE).expect("SixDaysLater.fng");
        let script = |object: &blackbox_feng::package::ObjectDef, id: u32| {
            object.scripts.iter().find(|script| script.id == id).cloned().expect("script")
        };
        let panel = package.objects.iter().find(|o| o.guid == 0xFA05F).expect("the panel");
        let text = package.objects.iter().find(|o| o.guid == 0xFA0AE).expect("the string");
        let (fade, hold) = (script(panel, 0x5079_C8F8), script(panel, 0x001C_A7C0));
        assert_eq!(fade.chain, Some(hold.id), "the panel holds after its fade");
        let send = hold.events.iter().find(|event| event.message == 0xEC5A_6043).expect("the panel's message");
        let (first, second) = (script(text, 0x5079_C8F8), script(text, 0x5A8E_4EBE));
        assert_eq!(first.chain, Some(second.id));
        let start = fade.length + send.time as i32;
        assert_eq!((start + first.length + second.length) as f32, CARD_TICKS);
        let string = text.string.as_ref().map(|s| s.text.clone()).unwrap_or_default();
        assert_eq!(string, CARD_TEXT, "the package's own text");
    }
}
