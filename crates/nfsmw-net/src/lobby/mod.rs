//! The lobby (MP-21). This part holds the token buckets of the per-player limits of `security-gameplay.md` section
//! 5.1, which chat, votes and the admission use as well.

mod rate;

#[cfg(test)]
mod tests;

pub use rate::{CHAT_RATE, JOIN_RATE, LOBBY_RATE, RateLimit, RateSpec, VOTE_START_RATE};
