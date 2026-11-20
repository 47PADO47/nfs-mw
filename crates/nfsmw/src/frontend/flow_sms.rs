//! A text message that the phone delivers in free roam opens over the game (`docs/specs/story-flow-2.md` section 3.7):
//! the world asks for it (`CareerRequest::Sms`), the game freezes and, as in the original, the inbox
//! (`SMS_MailBoxes.fng`) opens at that message and opens `SMS_Message.fng` over itself, so the inbox's logo bar, header,
//! brackets and footer bar frame the message (`docs/specs/fe-package-footer.md` section 4). Done goes back to the inbox,
//! whose Back resumes the game. The inbox key opens the inbox alone.

use super::*;
use crate::frontend::ids::screen::SMS_MAILBOXES;

impl Frontend {
    /// The delete question was answered Yes: the message's slot is emptied, then the screen closes as `then` says. `then`
    /// is often a screen change (Pop, or Switch back to the safe house), which the screen stack runs, not [`Self::apply`].
    pub(super) fn delete_sms(&mut self, id: u32, then: Option<Command>, host: &mut Host, env: &mut Env) {
        log::info!("career: SMS {id} deleted");
        crate::frontend::career_screens::sms::delete(id);
        let rest = self.screens.apply_stack_commands(then.into_iter().collect(), env);
        self.apply(rest, host, env);
    }

    /// The inbox key (d-pad right, Tab) in free roam: the phone's inbox opens over the frozen game, as the original HUD's
    /// right pad action does outside races (`docs/specs/fe-package-footer.md` section 4). Its Back resumes the game.
    pub(in crate::frontend) fn poll_inbox(
        &mut self,
        driving: bool,
        actions: &crate::input::ActionState,
        script_key: Option<bevy_input::keyboard::KeyCode>,
        host: &mut Host,
        env: &mut Env,
    ) {
        let pressed =
            actions.just_pressed(crate::input::Action::Inbox) || script_key == Some(bevy_input::keyboard::KeyCode::Tab);
        if !driving || !pressed || host.scene.race_active() || !crate::career::CareerHandle::shared().is_loaded() {
            return;
        }
        log::info!("phone: the inbox opens over the game");
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        self.screens.set_in_game();
        let args = Args { from_world: true, ..Args::default() };
        let commands = self.screens.open(SMS_MAILBOXES, args, true, env);
        self.apply(commands, host, env);
    }

    /// Opens message `id` over the frozen game, with the stage `Paused` so the screens take the pad: the inbox at that
    /// message, which opens it. The message is marked read when its screen starts.
    pub(super) fn open_sms(&mut self, id: u32, host: &mut Host, env: &mut Env) {
        log::info!("phone: message {id} opens over the game");
        self.latch = true;
        self.paused.set(true);
        self.stage = Stage::Paused;
        self.screens.set_in_game();
        crate::frontend::career_screens::sms::open_at(id);
        let args = Args { from_world: true, ..Args::default() };
        let commands = self.screens.open(SMS_MAILBOXES, args, true, env);
        // The inbox asks for the message as it starts: the stack pushes it.
        let rest = self.screens.apply_stack_commands(commands, env);
        self.apply(rest, host, env);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use blackbox_feng::runtime::pad;
    use game_install::GameDir;

    use super::*;
    use crate::frontend::ids::screen::SMS_MESSAGE;
    use crate::settings::{Partial, Settings, test_caps};
    use crate::ui::{Catalog, SCREEN_FILES, UiAssets};

    const FRAME: f32 = 1.0 / 60.0;

    /// The delivery of message 15 while driving: the inbox at the message, which opens it over itself.
    fn open_popup(screens: &mut Screens, env: &mut Env) {
        crate::frontend::career_screens::sms::open_at(15);
        let args = Args { from_world: true, ..Args::default() };
        let commands = screens.open(SMS_MAILBOXES, args, true, env);
        screens.apply_stack_commands(commands, env);
        // The inbox opens the message once its package is up (its init message, 500 ms in).
        for _ in 0..60 {
            let commands = screens.update(FRAME, 0, env);
            screens.apply_stack_commands(commands, env);
        }
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_popup_shows_its_message_and_back_resumes_the_game() {
        // The pending message of the inbox is shared state: one inbox test at a time.
        let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets);
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };

        open_popup(&mut screens, &mut env);
        assert_eq!(screens.top(), Some(SMS_MESSAGE), "the message screen is up, over the inbox");
        let back = |screens: &mut Screens, env: &mut Env| {
            for _ in 0..90 {
                screens.update(FRAME, 0, env);
            }
            let mut commands = Vec::new();
            for _ in 0..3 {
                commands.extend(screens.update(FRAME, pad::BACK, env));
            }
            for _ in 0..180 {
                commands.extend(screens.update(FRAME, 0, env));
            }
            screens.apply_stack_commands(commands, env)
        };
        back(&mut screens, &mut env);
        assert!(
            screens.top().is_some_and(|top| top.eq_ignore_ascii_case(SMS_MAILBOXES)),
            "Done goes back to the inbox"
        );
        let commands = back(&mut screens, &mut env);
        assert!(commands.iter().any(|c| matches!(c, Command::Resume)), "the inbox's Back resumes: {commands:?}");
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_popup_draws_its_packages_footer_for_the_device_used_last() {
        use crate::input::{Bindings, InputDevice};
        use crate::ui::input_icons::ATLAS;
        use blackbox_feng::NodeKind;
        // `SMS_Message.fng` has the keyboard's Done, Delete and Re-Play groups and the pad's, on the same line: the
        // package's own copy for the device is drawn, the other copy is not, and the host adds nothing.
        const KEYS: [u32; 3] = [0x523250e9, 0xc1c190eb, 0x914614e5];
        const PAD: [u32; 3] = [0x1c7fcf8b, 0xd18b818d, 0x2a631207];
        // The pending message of the inbox is shared state: one inbox test at a time.
        let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets);
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
        open_popup(&mut screens, &mut env);
        for _ in 0..120 {
            screens.update(FRAME, 0, &mut env);
        }
        for device in [InputDevice::Keyboard, InputDevice::Xbox, InputDevice::Keyboard] {
            let raw = screens.trees().pop().unwrap();
            let shown = screens.presented_trees(&Bindings::default(), device).pop().unwrap();
            assert_eq!(shown.nodes.len(), raw.nodes.len(), "{device:?}: no host hints");
            let drawn = |group: u32| {
                let Some(root) = shown.nodes.iter().position(|n| n.name_hash == group) else { return false };
                shown.draw_order.iter().any(|&i| shown.nodes[i].parent == Some(root))
            };
            let (used, other) = match device {
                InputDevice::Keyboard => (KEYS, PAD),
                InputDevice::Xbox => (PAD, KEYS),
            };
            assert!(used.iter().all(|&g| drawn(g)), "{device:?}: the device's buttons are drawn");
            assert!(!other.iter().any(|&g| drawn(g)), "{device:?}: the other device's buttons are not");
            let glyphs = shown
                .draw_order
                .iter()
                .any(|&i| matches!(shown.nodes[i].kind, NodeKind::Image { texture, .. } if texture == ATLAS));
            assert_eq!(glyphs, device == InputDevice::Xbox, "{device:?}: pad glyphs only for the controller");
        }
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_body_is_wrapped_over_the_packages_lines_from_the_top() {
        use crate::frontend::career_screens::sms_message::BODY_LINES;
        // The pending message of the inbox is shared state: one inbox test at a time.
        let _guard = crate::career::SHARED_CAREER_TEST.lock().unwrap_or_else(|e| e.into_inner());
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(PathBuf::from(root)).unwrap();
        let assets = Arc::new(UiAssets::load(&dir).unwrap());
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        let mut screens = Screens::new(catalog, assets);
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        let mut env = Env { settings: &mut settings, changed: &mut changed, caps: test_caps::native(), mods: None };
        open_popup(&mut screens, &mut env);
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
