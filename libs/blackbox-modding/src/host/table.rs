//! The host table a mod receives in `bb_mod_create` (ADR 0005 sections 1 and 3), and the interface tables that
//! `query_interface` hands out.
//!
//! The loader builds one host table per mod. The table and the state it points to stay in boxes owned by the table, so
//! their addresses do not move while the mod is loaded. Each interface table is built for one mod too (its `ctx` names
//! that mod's registrations), and it is released when the mod unloads. The host functions are the only code the mod calls
//! back into; each one is wrapped in `catch_unwind`, so a panic in them never unwinds into the mod.

#![allow(unsafe_code)] // ADR 0005 section 8: `unsafe` is confined to `abi`, `host::table`, `host::loader` and `sdk::ffi`.

mod audio;
#[cfg(test)]
mod audio_tests;
mod camera;
#[cfg(test)]
mod camera_tests;
mod commands;
mod hud;
#[cfg(test)]
mod hud_tests;
mod i18n;
#[cfg(test)]
mod i18n_tests;
mod input;
mod map;
mod menus;
#[cfg(test)]
mod menus_tests;
mod slot;
#[cfg(test)]
mod tests;
mod time;
mod ui;
#[cfg(test)]
mod ui_tests;
mod vehicles;
#[cfg(test)]
mod vehicles_tests;
mod world;
#[cfg(test)]
mod world_tests;

use core::ffi::c_void;
use core::ptr;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::rc::Rc;

use crate::abi::{
    ABI_MAJOR, ABI_MINOR, BB_LOG_DEBUG, BB_LOG_ERROR, BB_LOG_INFO, BB_LOG_TRACE, BB_LOG_WARN, BbHost, BbIface,
    BbStatus, BbStr,
};

use super::offer::HostOffer;
use super::registry::InterfaceRegistry;

pub(crate) use audio::attach as attach_audio;
pub(crate) use camera::attach as attach_camera;
pub(crate) use commands::attach as attach_commands;
pub(crate) use hud::attach as attach_hud;
pub(crate) use i18n::attach as attach_i18n;
pub(crate) use input::attach as attach_input;
pub(crate) use map::attach as attach_map;
pub(crate) use menus::attach as attach_menus;
pub use slot::Attached;
pub(crate) use time::attach as attach_time;
pub(crate) use ui::attach as attach_ui;
pub(crate) use vehicles::attach as attach_vehicles;
pub(crate) use world::attach as attach_world;

pub use audio::AudioBackend;
pub use camera::{CameraBackend, CameraKind, CameraState};
pub use commands::{CommandHandler, CommandsBackend};
pub use hud::{HudBackend, MAX_MESSAGE_SECS, MessageStyle};
pub use i18n::{I18nArg, I18nBackend, MAX_ARGS as MAX_I18N_ARGS};
pub use input::{HotkeyHandler, InputBackend};
pub use map::{BlipSpec, MapBackend};
pub use menus::{MAX_CHOICES, MenuHandler, MenuKind, MenuRowSpec, MenusBackend};
pub use time::TimeBackend;
pub use ui::{MAX_PRIMITIVES, UiBackend, UiPrim, UiRect, UiThemeSpec, UiTokens};
pub use vehicles::{VehicleState, VehiclesBackend};
pub use world::{MAX_COP_SHARE, WorldBackend};

/// What a backend method returns: its value, or the status the mod sees.
pub type BackendResult<T> = Result<T, BbStatus>;

/// The state a host function reads through the table's `ctx`: which mod is calling, the game's interfaces, and the
/// interface tables already handed to this mod.
struct CallState {
    mod_id: String,
    registry: Rc<InterfaceRegistry>,
    attached: RefCell<Vec<Attached>>,
}

impl CallState {
    /// The header of interface `id` for this mod, built on first use. `None` when the registry does not offer it at
    /// `major` with a minor of at least `min_minor`.
    fn attach(&self, id: &str, major: u16, min_minor: u16) -> Option<*const BbIface> {
        let known =
            self.attached.borrow().iter().find(|table| table.id() == id).map(|table| (table.header(), table.version()));
        if let Some((header, (have_major, have_minor))) = known {
            return (have_major == major && have_minor >= min_minor).then_some(header);
        }
        let built = self.registry.build(&self.mod_id, id, major, min_minor)?;
        let header = built.header();
        self.attached.borrow_mut().push(built);
        Some(header)
    }
}

/// The host table of one mod. Owns the table, the state its `ctx` points to, and the strings its game fields point
/// to. Dropping it removes what the mod registered through its interfaces, so the loader drops it only after the mod
/// is destroyed.
pub(crate) struct HostTable {
    _state: Box<CallState>,
    _strings: Box<[String; 4]>,
    table: Box<BbHost>,
}

impl HostTable {
    /// Build the table for mod `mod_id` living in `mod_dir`, answering with the identity of `offer` and the interfaces
    /// of `registry`.
    pub(crate) fn new(mod_id: &str, mod_dir: &Path, offer: &HostOffer, registry: Rc<InterfaceRegistry>) -> Self {
        let state = Box::new(CallState { mod_id: mod_id.to_owned(), registry, attached: RefCell::new(Vec::new()) });
        let strings = Box::new([
            offer.game_id.clone(),
            offer.game_version.clone(),
            mod_dir.to_string_lossy().into_owned(),
            offer.game_dir.clone(),
        ]);
        let ctx: *const CallState = &*state;
        let table = Box::new(BbHost {
            size: core::mem::size_of::<BbHost>() as u32,
            abi_major: ABI_MAJOR,
            abi_minor: ABI_MINOR,
            ctx: ctx.cast_mut().cast::<c_void>(),
            game_id: BbStr::new(&strings[0]),
            game_version: BbStr::new(&strings[1]),
            log: Some(host_log),
            query_interface: Some(host_query),
            mod_dir: BbStr::new(&strings[2]),
            game_dir: BbStr::new(&strings[3]),
        });
        HostTable { _state: state, _strings: strings, table }
    }

    /// The address the mod receives in `bb_mod_create`.
    pub(crate) fn as_ptr(&self) -> *const BbHost {
        &*self.table
    }
}

/// Map a `BB_LOG_*` level to the `log` crate. An unknown level logs at info.
fn level_of(level: u32) -> log::Level {
    match level {
        BB_LOG_ERROR => log::Level::Error,
        BB_LOG_WARN => log::Level::Warn,
        BB_LOG_INFO => log::Level::Info,
        BB_LOG_DEBUG => log::Level::Debug,
        BB_LOG_TRACE => log::Level::Trace,
        _ => log::Level::Info,
    }
}

/// Read the `CallState` behind a host table's `ctx`. `None` for a null pointer.
///
/// # Safety
/// `ctx` must be null or the pointer a [`HostTable`] stored, and that table must still be alive.
unsafe fn state_of<'a>(ctx: *mut c_void) -> Option<&'a CallState> {
    if ctx.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees `ctx` is a live `CallState` (see `# Safety`).
    Some(unsafe { &*ctx.cast::<CallState>() })
}

/// The `log` host function: one line to the game log, tagged with the mod's id.
unsafe extern "C" fn host_log(ctx: *mut c_void, level: u32, msg: BbStr) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `ctx` is the table's own state pointer; the table lives as long as the mod (see `# Safety` of
        // `state_of`).
        let Some(state) = (unsafe { state_of(ctx) }) else {
            return;
        };
        // SAFETY: the mod passes a string that is valid for this call (ADR 0005 section 4).
        let Some(text) = (unsafe { msg.as_str() }) else {
            return;
        };
        log::log!(target: "blackbox_modding::mod", level_of(level), "[{}] {}", state.mod_id, text);
    }));
}

/// The `query_interface` host function. The interface is built for the calling mod on first use and kept while the
/// mod is loaded. An absent interface, or one of another major or too old a minor, gives null, which the SDK reports
/// as `None`.
unsafe extern "C" fn host_query(ctx: *mut c_void, id: BbStr, major: u16, min_minor: u16) -> *const BbIface {
    let found = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: `ctx` is the table's own state pointer (see `state_of`).
        let state = unsafe { state_of(ctx) }?;
        // SAFETY: the mod passes an id that is valid for this call.
        let id = unsafe { id.as_str() }?;
        state.attach(id, major, min_minor)
    }));
    found.ok().flatten().unwrap_or(ptr::null())
}
