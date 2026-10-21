//! A text message that the phone delivers in free roam opens over the game (`docs/specs/story-flow-2.md` section 3.7):
//! the world asks for it (`CareerRequest::Sms`), the game freezes and `SMS_Message.fng` shows the message. Closing the
//! screen resumes the game, as the pause menu's Resume does.

use super::*;
use crate::frontend::career_screens::sms_popup::SmsPopup;
use crate::frontend::ids::screen::SMS_MESSAGE;

impl Frontend {
    /// Opens message `id` over the frozen game, with the stage `Paused` so the screens take the pad. The message is
    /// marked read when its screen starts.
    pub(super) fn open_sms(&mut self, id: u32, host: &mut Host, env: &mut Env) {
        log::info!("phone: message {id} opens over the game");
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        let commands = self.screens.open_with(SMS_MESSAGE, Box::new(SmsPopup::new(id)), true, env);
        self.apply(commands, host, env);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use blackbox_feng::runtime::pad;
    use game_install::GameDir;

    use super::*;
    use crate::settings::{Partial, Settings, test_caps};
    use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

    const FRAME: f32 = 1.0 / 60.0;

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_popup_shows_its_message_and_back_resumes_the_game() {
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets);
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };

        screens.open_with(SMS_MESSAGE, Box::new(SmsPopup::new(15)), true, &mut env);
        assert_eq!(screens.top(), Some(SMS_MESSAGE), "the message screen is up");
        for _ in 0..90 {
            screens.update(FRAME, 0, &mut env);
        }
        let mut commands = Vec::new();
        for _ in 0..3 {
            commands.extend(screens.update(FRAME, pad::BACK, &mut env));
        }
        for _ in 0..180 {
            commands.extend(screens.update(FRAME, 0, &mut env));
        }
        assert!(commands.iter().any(|c| matches!(c, Command::Resume)), "Back asks to resume: {commands:?}");
    }
}
