//! A text message that the phone delivers in free roam opens over the game (`docs/specs/story-flow-2.md` section 3.7):
//! the world asks for it (`CareerRequest::Sms`), the game freezes and `SMS_Message.fng` shows the message. Closing the
//! screen resumes the game, as the pause menu's Resume does.

use super::*;
use crate::frontend::career_screens::sms_popup::SmsPopup;
use crate::frontend::ids::screen::SMS_MESSAGE;

impl Frontend {
    /// The delete question was answered Yes: the message's slot is emptied, then the screen closes as `then` says. `then`
    /// is often a screen change (Pop, or Switch back to the safe house), which the screen stack runs, not [`Self::apply`].
    pub(super) fn delete_sms(&mut self, id: u32, then: Option<Command>, host: &mut Host, env: &mut Env) {
        log::info!("career: SMS {id} deleted");
        crate::frontend::career_screens::sms::delete(id);
        let rest = self.screens.apply_stack_commands(then.into_iter().collect(), env);
        self.apply(rest, host, env);
    }

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

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_popup_draws_one_footer_for_the_device_used_last() {
        use crate::input::{Bindings, InputDevice};
        use crate::ui::input_icons::ATLAS;
        use blackbox_feng::NodeKind;
        // `SMS_Message.fng` has the keyboard's Done, Delete and Re-Play and the controller's, on the same line: both give
        // way to the drawn hints.
        const PACKAGE_HINTS: [u32; 4] = [0x914614e5, 0x523250e9, 0xc1c190eb, 0xb04b66c8];
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets);
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
        screens.open_with(SMS_MESSAGE, Box::new(SmsPopup::new(15)), true, &mut env);
        for _ in 0..120 {
            screens.update(FRAME, 0, &mut env);
        }
        for device in [InputDevice::Keyboard, InputDevice::Xbox, InputDevice::Keyboard] {
            let raw = screens.trees().pop().unwrap();
            let shown = screens.presented_trees(&Bindings::default(), device).pop().unwrap();
            let hints = &shown.nodes[raw.nodes.len()..];
            let says = |text: &str| hints.iter().any(|n| n.visible && n.text.as_deref() == Some(text));
            assert!(says("Done") && says("Delete") && says("Re-Play"), "{device:?}: the drawn footer");
            let glyphs = hints
                .iter()
                .any(|n| n.visible && matches!(n.kind, NodeKind::Image { texture, .. } if texture == ATLAS));
            assert_eq!(glyphs, device == InputDevice::Xbox, "{device:?}: pad glyphs only for the controller");
            for hash in PACKAGE_HINTS {
                assert!(
                    !shown.nodes.iter().any(|n| n.name_hash == hash && n.visible),
                    "{device:?}: {hash:#x} still shows"
                );
            }
        }
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_body_is_wrapped_over_the_packages_lines_from_the_top() {
        use crate::frontend::career_screens::sms_message::BODY_LINES;
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets);
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
        screens.open_with(SMS_MESSAGE, Box::new(SmsPopup::new(15)), true, &mut env);
        for _ in 0..120 {
            screens.update(FRAME, 0, &mut env);
        }
        let tree = screens.trees().pop().unwrap();
        let line = |at: usize| {
            let node = tree.nodes.iter().find(|n| n.guid == BODY_LINES[at]).expect("a body line");
            node.text.clone().unwrap_or_default()
        };
        let lines: Vec<String> = (0..BODY_LINES.len()).map(line).collect();
        assert!(!lines[0].trim().is_empty(), "the body starts on the top line: {lines:?}");
        assert!(
            lines.iter().filter(|l| !l.trim().is_empty()).count() > 1,
            "a long body takes several lines: {lines:?}"
        );
        // The text keeps its paragraph breaks as empty lines; the last line of the window stays empty for a short body.
        assert!(lines[BODY_LINES.len() - 1].trim().is_empty(), "the body is not pushed to the bottom line: {lines:?}");
    }
}
