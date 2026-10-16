//! The NFS: Most Wanted extension of the mod API (ADR 0005 section 8; catalogue in docs/modding/api-nfsmw.md).
//!
//! Three interfaces, `nfsmw.pursuit`, `nfsmw.career` and `nfsmw.races`, and the events of the `0x1000` range. They
//! use the generic rules of `blackbox-modding`: size-prefixed `#[repr(C)]` tables, `extern "C"` functions with a
//! `BbStatus` return, handles and strings that do not outlive a call. A mod gets them through
//! `ctx.nfsmw_pursuit()` and the other [`sdk::NfsmwExt`] methods; the game gets the trampolines through [`host`].
//!
//! - `abi` (default): the tables, ids, codes and payloads. Plain data only; see [`abi`].
//! - `sdk`: the safe wrappers and the event decoder a mod links; see [`sdk`].
//! - `host`: the backend traits the game implements and the table builders; see [`host`].
//!
//! Nothing here does game IO or touches Bevy. The crate names no install path; the data it carries (cop waves, cars)
//! comes from the game through the backends.

#[cfg(feature = "abi")]
pub mod abi;

#[cfg(feature = "sdk")]
pub mod sdk;

#[cfg(feature = "host")]
pub mod host;

#[cfg(all(test, feature = "sdk", feature = "host"))]
mod tests;
