//! The typed text of the screen on top (`ScreenLogic::wants_text`), for the alias name prompt.

use super::{Command, Env, Screens};
use crate::frontend::name_entry::TextKey;

impl Screens {
    /// Whether the top screen takes typed text (`ScreenLogic::wants_text`).
    pub fn wants_text(&self) -> bool {
        self.stack.last().is_some_and(|top| top.logic.wants_text())
    }

    /// Hands a typed key to the top screen's logic and returns what it queued.
    pub fn type_key(&mut self, key: TextKey, env: &mut Env) -> Vec<Command> {
        let Some((_, commands)) = self.on_top(env, |logic, cx| {
            logic.typed(cx, key);
            true
        }) else {
            return Vec::new();
        };
        self.apply_stack_commands(commands, env)
    }
}
