//! `bb.commands` 1.0: run a console line, and register a mod command with a callback.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbSinkFn, BbStatus, BbStr};

/// `run`: execute one console line (built-in, scene or mod command). The output goes to `sink`, once.
pub type BbCommandsRunFn =
    unsafe extern "C" fn(ctx: *mut c_void, line: BbStr, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus;

/// The callback of a registered command. `argv` holds `argc` words split from the line, without the command name. The
/// output goes to `sink`. A negative status is shown to the player as an error.
pub type BbCommandCallbackFn = unsafe extern "C" fn(
    user: *mut c_void,
    argv: *const BbStr,
    argc: u64,
    sink: Option<BbSinkFn>,
    sink_user: *mut c_void,
) -> BbStatus;

/// `register`: name and help text, the mod's `user` pointer and callback. The new command id is written to `out_id`.
/// The `user` pointer must stay valid until the command is unregistered.
pub type BbCommandsRegisterFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    name: BbStr,
    help: BbStr,
    user: *mut c_void,
    callback: Option<BbCommandCallbackFn>,
    out_id: *mut u64,
) -> BbStatus;

/// `unregister`: remove a command by id. The host also removes every command of a mod when the mod unloads.
pub type BbCommandsUnregisterFn = unsafe extern "C" fn(ctx: *mut c_void, id: u64) -> BbStatus;

/// The `bb.commands` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbCommands {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Run a console line.
    pub run: Option<BbCommandsRunFn>,
    /// Register a command.
    pub register: Option<BbCommandsRegisterFn>,
    /// Unregister a command.
    pub unregister: Option<BbCommandsUnregisterFn>,
}
