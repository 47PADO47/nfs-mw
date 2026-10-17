//! `Commands`: run a console line, and register a Rust closure as a console command (catalogue: docs/modding/api.md#commands).

use super::ctx::Keep;
use super::ffi::HostTable;
use super::ffi::commands::{self as host_commands, CommandsFns};
use super::{CommandId, Ctx, ModResult};

/// The `bb.commands` interface of the host, as a mod uses it. Obtained from `Ctx::commands`.
pub struct Commands {
    fns: CommandsFns,
    host: HostTable,
    keep: Keep,
}

impl Commands {
    pub(crate) fn new(fns: CommandsFns, host: HostTable, keep: Keep) -> Self {
        Commands { fns, host, keep }
    }

    /// Run one console line (a built-in, scene or mod command) and return its output.
    pub fn run(&self, line: &str) -> ModResult<String> {
        host_commands::run(&self.fns, line)
    }

    /// Register a console command. The handler gets the words after the command name, and returns the output or an
    /// error. The command is removed when the mod unloads.
    pub fn register<F>(&self, name: &str, help: &str, handler: F) -> ModResult<CommandId>
    where
        F: FnMut(&mut Ctx, &[&str]) -> ModResult<String> + 'static,
    {
        host_commands::register(&self.fns, self.host, &self.keep, name, help, handler)
    }

    /// Remove a command this mod registered.
    pub fn unregister(&self, id: CommandId) -> ModResult<()> {
        host_commands::unregister(&self.fns, id)
    }
}
