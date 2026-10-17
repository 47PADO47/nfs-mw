//! The C ABI shared by the game and every mod library (ADR 0005 section 1).
//!
//! Only `#[repr(C)]` plain data, fixed-width integers, raw pointers and `extern "C"` function pointers cross
//! the boundary. No `usize` in a struct (lengths are `u64`), no `bool` (use `u8`), no Rust enum with a payload.
//!
//! Append-only rules:
//!
//! - Every table starts with `size: u32`, the number of bytes its builder filled in, then its version. A reader
//!   checks `size` against the end of a field before it reads that field (see [`has_field`]). A field the table
//!   does not cover means "not supported".
//! - Within one major version a table only grows. A field is never moved, retyped or removed. A removed call
//!   keeps its slot and answers `BB_ERR_REMOVED`.
//! - Status codes and event kinds are never reused. A new code takes the next unused value.
//! - A breaking change bumps the major version ([`ABI_MAJOR`]) and gets a new interface id suffix.
//! - Every layout change edits the snapshot numbers in `tests.rs` in the same commit.
//!
//! Function pointers in tables are `Option<fn>`. A null pointer is then a valid value, and the layout is the
//! same as the bare pointer (both are one pointer wide). The C side can fill a null for an absent function.

#![allow(unsafe_code)] // ADR 0005 section 8: `unsafe` is confined to `abi`, `host::loader` and `sdk::ffi`.

mod event;
mod handle;
mod host;
mod iface;
mod module;
mod size;
mod status;
mod string;
mod version;

#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod tests;

pub use event::{
    BB_COLLIDE_NONE, BB_COLLIDE_PROP, BB_COLLIDE_VEHICLE, BB_COLLIDE_WALL, BB_EVENT_ABILITY_ACTIVATED,
    BB_EVENT_ACTION_PRESSED, BB_EVENT_FRAME_START, BB_EVENT_HOUR_CHANGED, BB_EVENT_INTERIOR_ENTERED,
    BB_EVENT_INTERIOR_ENTERING, BB_EVENT_INTERIOR_EXITED, BB_EVENT_INTERIOR_EXITING, BB_EVENT_MENU_CLOSED,
    BB_EVENT_MENU_OPENED, BB_EVENT_MOD_UNLOADING, BB_EVENT_SCENE_TICK, BB_EVENT_VEHICLE_COLLISION,
    BB_EVENT_VEHICLE_DESPAWNED, BB_EVENT_VEHICLE_SPAWNED, BB_EVENT_WORLD_LOADED, BB_EVENT_WORLD_UNLOADING, BB_ROLE_COP,
    BB_ROLE_PLAYER, BB_ROLE_RACER, BB_ROLE_TRAFFIC, BbAbilityActivated, BbActionPressed, BbFrameStart, BbHourChanged,
    BbInteriorEvent, BbMenuEvent, BbModUnloading, BbSceneTick, BbVehicleCollision, BbVehicleDespawned,
    BbVehicleSpawned, BbWorldLoaded, BbWorldUnloading,
};
pub use handle::{BB_NULL_HANDLE, handle_generation, handle_index, make_handle};
pub use host::{
    BB_LOG_DEBUG, BB_LOG_ERROR, BB_LOG_INFO, BB_LOG_TRACE, BB_LOG_WARN, BbHost, BbLogFn, BbQueryInterfaceFn,
};
pub use iface::{
    BB_CAMERA_BUMPER, BB_CAMERA_CHASE, BB_CAMERA_CLOSE, BB_CAMERA_DRIFT, BB_CAMERA_DYNAMIC_FOV, BB_CAMERA_FAR,
    BB_CAMERA_HOOD, BB_CAMERA_PURSUIT, BB_CAMERA_SUPER_FAR, BB_HUD_BIG, BB_HUD_INFO, BB_HUD_WARNING, BbAudio,
    BbAudioPlayFn, BbCamera, BbCameraGetFn, BbCameraSetModeFn, BbCameraState, BbHud, BbHudHideFn, BbHudMessageFn,
    BbVec3, BbVehicleRepairFn, BbVehicleSpawnTrafficFn, BbVehicleState, BbVehicleStateFn, BbVehicleTeleportFn,
    BbVehicles, BbWorld, BbWorldSetRainFn, BbWorldSetTrafficFn, BbWorldTextFn, IFACE_AUDIO, IFACE_AUDIO_VERSION,
    IFACE_CAMERA, IFACE_CAMERA_VERSION, IFACE_HUD, IFACE_HUD_VERSION, IFACE_VEHICLES, IFACE_VEHICLES_VERSION,
    IFACE_WORLD, IFACE_WORLD_VERSION,
};
pub use iface::{
    BB_MENU_BUTTON, BB_MENU_LIST, BB_MENU_SLIDER, BB_MENU_TOGGLE, BbBlip, BbCommandCallbackFn, BbCommands,
    BbCommandsRegisterFn, BbCommandsRunFn, BbCommandsUnregisterFn, BbHotkeyFn, BbIface, BbInput, BbInputRegisterFn,
    BbInputUnregisterFn, BbMap, BbMapAddFn, BbMapRemoveFn, BbMapUpdateFn, BbMenuCallbackFn, BbMenuRow, BbMenus,
    BbMenusGetFn, BbMenusRegisterFn, BbMenusSetFn, BbMenusUnregisterFn, BbRgba, BbTime, BbTimeFreezeFn, BbTimeGetFn,
    BbTimeOfDay, BbTimeSetDayLengthFn, BbTimeSetHourFn, IFACE_COMMANDS, IFACE_COMMANDS_VERSION, IFACE_INPUT,
    IFACE_INPUT_VERSION, IFACE_MAP, IFACE_MAP_VERSION, IFACE_MENUS, IFACE_MENUS_VERSION, IFACE_TIME,
    IFACE_TIME_VERSION,
};
pub use module::{
    BB_EVENT_CANCEL, BB_EVENT_CONTINUE, BbDestroyFn, BbEvent, BbEventResult, BbModApi, BbModCreateFn, BbModInfo,
    BbModInfoFn, BbOnEventFn, MOD_CREATE_SYMBOL, MOD_INFO_SYMBOL,
};
pub use size::has_field;
pub use status::{
    BB_ERR_ABI, BB_ERR_ARG, BB_ERR_DENIED, BB_ERR_EXISTS, BB_ERR_HOST, BB_ERR_NOT_FOUND, BB_ERR_NOT_IN_CALLBACK,
    BB_ERR_NOT_READY, BB_ERR_PANIC, BB_ERR_PHASE, BB_ERR_REMOVED, BB_ERR_STALE, BB_ERR_UNSUPPORTED,
    BB_ERR_WRONG_THREAD, BB_OK, BbStatus,
};
pub use string::{BbSinkFn, BbStr, MAX_MESSAGE_BYTES, MAX_NAME_BYTES, MAX_PATH_BYTES};
pub use version::{ABI_MAJOR, ABI_MINOR, ABI_VERSION, AbiMismatch, AbiVersion, check_handshake};
