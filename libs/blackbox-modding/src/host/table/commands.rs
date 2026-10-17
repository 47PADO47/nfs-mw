//! `bb.commands` on the host side: the backend the game implements, and the trampolines behind `BbCommands`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, caught, guard, name_of, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BB_OK, BbCommandCallbackFn, BbCommands, BbIface, BbSinkFn, BbStatus, BbStr, IFACE_COMMANDS,
    IFACE_COMMANDS_VERSION, MAX_MESSAGE_BYTES,
};

/// The Rust side of a mod command: the words after the command name go in, the output comes out, or a status.
pub type CommandHandler = Box<dyn Fn(&[&str]) -> BackendResult<String>>;

/// What the game implements for `bb.commands`. `owner` is the id of the mod that made the call.
pub trait CommandsBackend {
    /// Run one console line (a built-in, scene or mod command) and return its output.
    fn run(&self, owner: &str, line: &str) -> BackendResult<String>;
    /// Register a command. `handler` calls the mod's callback. Returns the command's handle.
    fn register(&self, owner: &str, name: &str, help: &str, handler: CommandHandler) -> BackendResult<u64>;
    /// Remove a command by handle.
    fn unregister(&self, owner: &str, handle: u64) -> BackendResult<()>;
}

/// Build the `bb.commands` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn CommandsBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbCommands {
        head: BbIface {
            size: size_of::<BbCommands>() as u32,
            major: IFACE_COMMANDS_VERSION.0,
            minor: IFACE_COMMANDS_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        run: Some(run),
        register: Some(register),
        unregister: Some(unregister),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.unregister(owner, handle);
        })
    });
    Attached::new(IFACE_COMMANDS, IFACE_COMMANDS_VERSION, table, release)
}

/// `run`: one console line, output through the caller's sink.
unsafe extern "C" fn run(ctx: *mut c_void, line: BbStr, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CommandsBackend>(ctx)? };
        let sink = sink.ok_or(BB_ERR_ARG)?;
        // SAFETY: the caller's string is valid for this call.
        let line = unsafe { line.as_str() }.ok_or(BB_ERR_ARG)?;
        let output = slot.backend.run(&slot.owner, line)?;
        // SAFETY: the sink is the caller's function, and `output` is valid for the duration of the call.
        unsafe { sink(sink_user, BbStr::new(&output)) };
        Ok(())
    })
}

/// `register`: a command with a name, help text and the mod's callback.
unsafe extern "C" fn register(
    ctx: *mut c_void,
    name: BbStr,
    help: BbStr,
    user: *mut c_void,
    callback: Option<BbCommandCallbackFn>,
    out_id: *mut u64,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CommandsBackend>(ctx)? };
        let callback = callback.ok_or(BB_ERR_ARG)?;
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the caller's strings are valid for this call.
        let name = unsafe { name_of(name) }?;
        // SAFETY: as above.
        let help = unsafe { text_within(help, MAX_MESSAGE_BYTES) }?;
        let handler: CommandHandler = Box::new(move |args: &[&str]| {
            // SAFETY: the mod keeps `user` valid until the command is unregistered (ADR 0005 section 4).
            unsafe { call_command(callback, user, args) }
        });
        let handle = slot.backend.register(&slot.owner, name, help, handler)?;
        slot.remember(handle);
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `unregister`: remove one command.
unsafe extern "C" fn unregister(ctx: *mut c_void, handle: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn CommandsBackend>(ctx)? };
        slot.backend.unregister(&slot.owner, handle)?;
        slot.forget(handle);
        Ok(())
    })
}

/// Call a mod command with `args`, collecting its output.
///
/// # Safety
/// `user` must be the pointer the mod gave for this command, still valid (the command is registered).
unsafe fn call_command(callback: BbCommandCallbackFn, user: *mut c_void, args: &[&str]) -> BackendResult<String> {
    let words: Vec<BbStr> = args.iter().map(|word| BbStr::new(word)).collect();
    let mut output = String::new();
    let sink_user: *mut c_void = (&mut output as *mut String).cast();
    // SAFETY: `words` holds `argc` strings valid for the call; `sink_user` is the `output` above, alive for the call.
    let status =
        caught(|| unsafe { Ok(callback(user, words.as_ptr(), words.len() as u64, Some(collect), sink_user)) })?;
    match status {
        BB_OK => Ok(output),
        failed => Err(failed),
    }
}

/// The sink a host-side call hands to a mod command: appends the text to the `String` it names.
unsafe extern "C" fn collect(user: *mut c_void, text: BbStr) {
    // SAFETY: `user` is the `String` that `call_command` passed, alive for the call.
    let Some(output) = (unsafe { user.cast::<String>().as_mut() }) else {
        return;
    };
    // SAFETY: the mod passes a string valid for this call.
    let Some(text) = (unsafe { text.as_str() }) else {
        return;
    };
    output.push_str(text);
}
