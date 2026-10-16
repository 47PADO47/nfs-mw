//! Status codes. A call returns `BB_OK` (0) or a negative `BB_ERR_*` code. Codes are never reused.

/// The status a fallible call returns across the boundary.
pub type BbStatus = i32;

/// The call succeeded.
pub const BB_OK: BbStatus = 0;
/// A bad argument: a null pointer, a bad length or a field the table does not cover.
pub const BB_ERR_ARG: BbStatus = -1;
/// The two sides speak different ABI major versions.
pub const BB_ERR_ABI: BbStatus = -2;
/// The host does not offer the requested call or interface.
pub const BB_ERR_UNSUPPORTED: BbStatus = -3;
/// A handle refers to an object that no longer exists (despawned, or from an earlier world).
pub const BB_ERR_STALE: BbStatus = -4;
/// The call is not allowed in the current phase (for example content registration after start-up).
pub const BB_ERR_PHASE: BbStatus = -5;
/// The call is valid but not yet possible (for example an ability still cooling down).
pub const BB_ERR_NOT_READY: BbStatus = -6;
/// A host call was made outside a mod callback.
pub const BB_ERR_NOT_IN_CALLBACK: BbStatus = -7;
/// A host call was made from a thread other than the main thread.
pub const BB_ERR_WRONG_THREAD: BbStatus = -8;
/// The host refused the call (multiplayer guard, or the user's settings).
pub const BB_ERR_DENIED: BbStatus = -9;
/// A named object does not exist.
pub const BB_ERR_NOT_FOUND: BbStatus = -10;
/// A named object already exists.
pub const BB_ERR_EXISTS: BbStatus = -11;
/// The call is still in the table but has been removed from the ABI; it does nothing.
pub const BB_ERR_REMOVED: BbStatus = -12;
/// A panic was caught at the boundary. It never unwinds into the host.
pub const BB_ERR_PANIC: BbStatus = -13;
/// The host reported an error; its message is read through the host's last-error call.
pub const BB_ERR_HOST: BbStatus = -14;
