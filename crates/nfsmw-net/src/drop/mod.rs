//! Disconnects (MP-50), reconnects (MP-51) and the late-join state transfer (MP-53).
//!
//! - [`Presence`]: when each slot was last heard from. Silence of [`LAG_MS`] marks a peer lagging (its car is parked by
//!   the input window); silence of [`TIMEOUT_MS`] (10 s) drops it.
//! - [`on_drop`], [`on_expiry`], [`on_resume`]: the [`DropAction`]s the server carries out, in order: the pursuit loses
//!   the target, a timed-out player's car stays as a parked non-colliding ghost and the slot is held for
//!   [`RESUME_WINDOW_MS`] (30 s), a quit or an expired window removes the car, scores a racer as not finished and frees
//!   the slot. A kick or ban never holds the slot.
//! - [`ResumeBook`]: single-use resume tokens bound to the player id, kept as hashes, compared in constant time.
//! - [`BulkSender`], [`BulkReceiver`] and [`SessionData`]: the session data a late joiner receives on the bulk channel,
//!   split into ordered chunks under a byte budget per call and reassembled with length and hash checks.

mod actions;
mod bulk;
mod presence;
mod resume;
mod sync;

#[cfg(test)]
mod tests;

pub use actions::{DropAction, DropCause, DropContext, on_drop, on_expiry, on_resume};
pub use bulk::{BulkError, BulkReceiver, BulkSender, Complete};
pub use presence::{LAG_MS, Liveness, Presence, TIMEOUT_MS};
pub use resume::{RESUME_TOKEN_BYTES, RESUME_WINDOW_MS, ResumeBook, ResumeError, Resumed};
pub use sync::{LoadoutRow, SESSION_DATA_LIMIT, SessionData};
