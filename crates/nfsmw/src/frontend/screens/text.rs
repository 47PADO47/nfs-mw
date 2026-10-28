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
        // Text editing is independent of releasing the action that opened the prompt. Its Accept/Back still use
        // the navigation latch in update; pending host transitions must reject both text and navigation.
        let Some(top) = self.stack.last().filter(|top| top.logic.accepts_input()).map(|top| top.id) else {
            return Vec::new();
        };
        let mut commands = Vec::new();
        self.with_logic(top, env, &mut commands, |logic, cx| {
            logic.typed(cx, key);
        });
        self.apply_stack_commands(commands, env)
    }
}
