//! `bb.commands` on the mod side: read the table, run a console line, and register a Rust closure as a console command.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{HostTable, finish, read_or, status_result};
use crate::abi::{
    BB_ERR_ARG, BB_ERR_NOT_READY, BB_OK, BbCommands, BbCommandsRegisterFn, BbCommandsRunFn, BbCommandsUnregisterFn,
    BbSinkFn, BbStatus, BbStr,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{CommandId, Ctx, Error, Interface, ModResult, guard};

/// The function pointers of one `bb.commands` table, read once. Each is `None` when the host does not provide it.
#[derive(Clone, Copy)]
pub(crate) struct CommandsFns {
    ctx: *mut c_void,
    run: Option<BbCommandsRunFn>,
    register: Option<BbCommandsRegisterFn>,
    unregister: Option<BbCommandsUnregisterFn>,
}

/// Read the `bb.commands` table the host offered.
pub(crate) fn read(iface: Interface) -> CommandsFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        CommandsFns {
            ctx: read_or(&iface, offset_of!(BbCommands, ctx), null_mut()),
            run: read_or(&iface, offset_of!(BbCommands, run), None),
            register: read_or(&iface, offset_of!(BbCommands, register), None),
            unregister: read_or(&iface, offset_of!(BbCommands, unregister), None),
        }
    }
}

/// Run one console line and return its output.
pub(crate) fn run(fns: &CommandsFns, line: &str) -> ModResult<String> {
    let run = fns.run.ok_or(Error::Unsupported)?;
    let mut output = String::new();
    let sink_user: *mut c_void = (&mut output as *mut String).cast();
    // SAFETY: the host calls `capture` only during this call, with `sink_user` naming `output`, which outlives it.
    let status = unsafe { run(fns.ctx, BbStr::new(line), Some(capture), sink_user) };
    status_result(status)?;
    Ok(output)
}

/// The sink of [`run`]: appends the host's text to the `String` it names.
unsafe extern "C" fn capture(user: *mut c_void, text: BbStr) {
    // SAFETY: `user` is the `String` that `run` passed in, alive for the call.
    let Some(output) = (unsafe { user.cast::<String>().as_mut() }) else {
        return;
    };
    // SAFETY: the host passes a string valid for this call.
    let Some(text) = (unsafe { text.as_str() }) else {
        return;
    };
    output.push_str(text);
}

/// A registered command: the closure, and what its trampoline needs to call it.
struct CommandEntry<F> {
    host: HostTable,
    keep: Keep,
    handler: RefCell<F>,
}

/// Register a closure as a console command. The closure is kept in `keep` until the mod unloads.
pub(crate) fn register<F>(
    fns: &CommandsFns,
    host: HostTable,
    keep: &Keep,
    name: &str,
    help: &str,
    handler: F,
) -> ModResult<CommandId>
where
    F: FnMut(&mut Ctx, &[&str]) -> ModResult<String> + 'static,
{
    let register = fns.register.ok_or(Error::Unsupported)?;
    let entry = Box::new(CommandEntry { host, keep: keep.clone(), handler: RefCell::new(handler) });
    let user: *mut c_void = (&*entry as *const CommandEntry<F>).cast_mut().cast();
    let mut id = 0_u64;
    // SAFETY: the host calls `command_trampoline::<F>` with `user` only while the command is registered. `entry` moves
    // into `keep` below, which lives until the mod's `destroy`; the host removes the command when the mod unloads.
    let status =
        unsafe { register(fns.ctx, BbStr::new(name), BbStr::new(help), user, Some(command_trampoline::<F>), &mut id) };
    status_result(status)?;
    keep.borrow_mut().push(entry);
    Ok(CommandId::from_raw(id))
}

/// Remove a command this mod registered.
pub(crate) fn unregister(fns: &CommandsFns, id: CommandId) -> ModResult<()> {
    let unregister = fns.unregister.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, id.raw()) })
}

/// The callback the host calls for a command the SDK registered: runs the closure in `user`.
unsafe extern "C" fn command_trampoline<F>(
    user: *mut c_void,
    argv: *const BbStr,
    argc: u64,
    sink: Option<BbSinkFn>,
    sink_user: *mut c_void,
) -> BbStatus
where
    F: FnMut(&mut Ctx, &[&str]) -> ModResult<String> + 'static,
{
    if user.is_null() {
        return BB_ERR_ARG;
    }
    // SAFETY: `user` is the `CommandEntry<F>` that `register` kept alive (see there).
    let entry = unsafe { &*user.cast::<CommandEntry<F>>() };
    // SAFETY: the host passes `argc` strings, valid for this call.
    let Some(words) = (unsafe { read_words(argv, argc) }) else {
        return BB_ERR_ARG;
    };
    // A command that runs itself (through `run`) finds its closure busy, and is refused instead of aliased.
    let Ok(mut handler) = entry.handler.try_borrow_mut() else {
        return BB_ERR_NOT_READY;
    };
    let mut ctx = Ctx::new(entry.host, entry.keep.clone());
    let result = guard::run(|| (*handler)(&mut ctx, &words));
    match finish(entry.host, "command", result) {
        Ok(text) => {
            if let Some(sink) = sink {
                // SAFETY: the host gave the sink and its user pointer for this call, and `text` is valid for the call.
                unsafe { sink(sink_user, BbStr::new(&text)) };
            }
            BB_OK
        }
        Err(error) => error.code(),
    }
}

/// Read `argc` strings from `argv`. `None` when the pointer is null for a non-zero count, or a string is not UTF-8.
///
/// # Safety
/// `argv` must be valid for reads of `argc` strings, each valid for the call, or `argc` must be zero.
unsafe fn read_words<'a>(argv: *const BbStr, argc: u64) -> Option<Vec<&'a str>> {
    if argc == 0 {
        return Some(Vec::new());
    }
    if argv.is_null() {
        return None;
    }
    let count = usize::try_from(argc).ok()?;
    let mut words = Vec::with_capacity(count);
    for index in 0..count {
        // SAFETY: `index` is below `argc`, and the host passes `argc` strings at `argv` (see `# Safety`).
        let word = unsafe { argv.add(index).read() };
        // SAFETY: each string is valid for the call (see `# Safety`).
        words.push(unsafe { word.as_str() }?);
    }
    Some(words)
}
