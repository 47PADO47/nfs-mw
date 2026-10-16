//! `bb.commands`: the console commands a mod registers, and the console lines it runs.
//!
//! A line a mod runs goes into the console's own queue, so it is parsed and run as a typed line is, from the next frame
//! on. Its output goes to the console log, not back to the mod. A command a mod registers is consulted after the
//! built-in commands and before the scene's, so it cannot shadow a built-in; it shows in `help` under its name.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{BackendResult, CommandHandler, CommandsBackend};

use super::bridge::Bridge;
use super::is_plain_name;

/// The `bb.commands` port of the game: one per mod table, sharing the bridge.
pub struct CommandsPort(pub(super) Rc<Bridge>);

/// A console command a mod registered.
pub struct ModCommand {
    pub handle: u64,
    pub owner: String,
    /// Lower case: the console lower-cases the word it reads.
    pub name: String,
    pub help: String,
    /// The mod's callback. Shared, so a call can run while the table is free to change.
    pub handler: Rc<CommandHandler>,
}

impl CommandsBackend for CommandsPort {
    fn run(&self, _owner: &str, line: &str) -> BackendResult<String> {
        let line = line.trim();
        if line.is_empty() {
            return Err(BB_ERR_ARG);
        }
        self.0.queued_lines.borrow_mut().push(line.to_owned());
        Ok(String::new())
    }

    fn register(&self, owner: &str, name: &str, help: &str, handler: CommandHandler) -> BackendResult<u64> {
        if !is_plain_name(name) {
            return Err(BB_ERR_ARG);
        }
        let name = name.to_ascii_lowercase();
        let mut commands = self.0.commands.borrow_mut();
        if commands.iter().any(|command| command.name == name) {
            return Err(BB_ERR_EXISTS);
        }
        let handle = self.0.next_handle();
        commands.push(ModCommand {
            handle,
            owner: owner.to_owned(),
            name,
            help: help.to_owned(),
            handler: Rc::new(handler),
        });
        Ok(handle)
    }

    fn unregister(&self, owner: &str, handle: u64) -> BackendResult<()> {
        let mut commands = self.0.commands.borrow_mut();
        let before = commands.len();
        commands.retain(|command| !(command.handle == handle && command.owner == owner));
        if commands.len() == before {
            return Err(BB_ERR_NOT_FOUND);
        }
        Ok(())
    }
}

/// The mod commands as `help` lists them: the usage (the name), and the help text with the mod's id.
pub fn listing(bridge: &Bridge) -> Vec<(String, String)> {
    bridge
        .commands
        .borrow()
        .iter()
        .map(|command| (command.name.clone(), format!("{} (mod {})", command.help, command.owner)))
        .collect()
}

/// Whether a mod registered a command called `name` (lower case).
pub fn is_registered(bridge: &Bridge, name: &str) -> bool {
    bridge.commands.borrow().iter().any(|command| command.name == name)
}

/// Run the mod command `name` with `args`, and give its output. The handler is cloned out before the call, so the mod
/// may register or remove commands while it runs.
pub fn run_mod_command(bridge: &Bridge, name: &str, args: &[&str]) -> Result<String, String> {
    let handler = {
        let commands = bridge.commands.borrow();
        let command =
            commands.iter().find(|command| command.name == name).ok_or_else(|| format!("unknown command {name:?}"))?;
        command.handler.clone()
    };
    (*handler)(args).map_err(|status| format!("{name} failed (status {status})"))
}
