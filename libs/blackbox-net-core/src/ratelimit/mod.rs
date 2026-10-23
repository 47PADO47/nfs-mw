//! Rate limits on a caller-driven clock (security-ops.md sections 2.1 to 2.4, security.md 4.6). Standard library only,
//! so it is in every build; every function that needs the time takes `now_ms`.
//!
//! - [`TokenBucket`]: burst and refill with integer arithmetic.
//! - [`KeyedLimiter`]: one bucket per key (address, peer), bounded, failing closed when full of active keys.
//! - [`FailureLimiter`]: failures per key with a doubling lock ([`FailureRule::PASSWORD`], [`FailureRule::AUTH`]).
//! - [`PreAuthLimits`] and [`PreAuthGate`]: the per-address and global caps in front of the handshake and discovery;
//!   [`PeerBudget`]: the inbound byte and datagram budget of one connection.
//!
//! Over a limit, the caller drops silently: no refusal packet, which could itself be an amplifier or an oracle.

mod bucket;
mod failures;
mod keyed;
mod preauth;
#[cfg(test)]
mod tests;

pub use bucket::TokenBucket;
pub use failures::{FailureLimiter, FailureRule};
pub use keyed::KeyedLimiter;
pub use preauth::{PeerBudget, PreAuthGate, PreAuthLimits};
