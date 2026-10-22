//! Text chat rules (MP-40), quick messages and emotes (MP-44), and system notices.
//!
//! The server runs every `ChatSend` and `QuickMessage` through a [`ChatGate`]: a server mute (the host's, MP-42) drops
//! the line without spending a token, then the chat bucket (1 line per second, burst 3, shared with quick messages),
//! then a whisper's target must be another member, then [`normalise`] cleans the text again (the decoder only refused
//! control and bidirectional characters; the gate also strips invisible characters and combining floods). The answer
//! is a [`Relay`] naming the [`Audience`], or a [`ChatRefusal`] the caller may tell the sender about.
//!
//! [`notice`] builds the `SystemNotice` lines for joins, leaves, kicks, mutes, votes and results. A client's own mute
//! list is [`crate::roles::LocalMutes`].
//!
//! Quick messages are ids into a table the game owns: preset lines `0..QUICK_LINES` and emotes
//! `EMOTE_BASE..EMOTE_BASE + EMOTES`. The catalogue has no server-to-client quick message yet, so the server crate
//! relays a [`QuickRelay`] in its own way (a gap of the message catalogue).

mod normalise;
pub mod notice;

#[cfg(test)]
mod tests;

pub use normalise::{MAX_CHAT_CHARS, MAX_COMBINING_RUN, normalise};

use std::collections::BTreeMap;

use crate::lobby::{CHAT_RATE, RateLimit};
use crate::message::{ChatChannel, ChatMessage, ChatSend, QuickMessage};

/// Preset quick lines: ids `0..QUICK_LINES`.
pub const QUICK_LINES: u16 = 64;
/// First emote id.
pub const EMOTE_BASE: u16 = 0x100;
/// Emotes: ids `EMOTE_BASE..EMOTE_BASE + EMOTES`.
pub const EMOTES: u16 = 32;

/// Whether `id` names a preset line or an emote.
pub fn is_quick_id(id: u16) -> bool {
    id < QUICK_LINES || (EMOTE_BASE..EMOTE_BASE + EMOTES).contains(&id)
}

/// Who sent a line, as the lobby knows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sender {
    pub slot: u8,
    pub team: u8,
    /// Muted by the host or an admin.
    pub muted: bool,
}

/// Who receives a relayed line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Audience {
    /// Every member.
    All,
    /// Members of this team.
    Team(u8),
    /// The sender and the target only.
    Whisper { from: u8, to: u8 },
}

/// A line the server sends on.
#[derive(Debug, Clone, PartialEq)]
pub struct Relay {
    pub message: ChatMessage,
    pub audience: Audience,
}

/// A quick message the server sends on to every member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuickRelay {
    pub from: u8,
    pub id: u16,
}

/// Why a line was not relayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatRefusal {
    /// The sender is muted by the server.
    Muted,
    /// Over the chat rate; tell the sender.
    RateLimited,
    /// A whisper to nobody or to oneself.
    UnknownTarget,
    /// Nothing printable was left after [`normalise`].
    Empty,
    /// Not a quick line or emote id.
    UnknownQuick,
}

/// The server's chat gate: one bucket per slot, kept until [`ChatGate::forget`].
#[derive(Debug, Clone, Default)]
pub struct ChatGate {
    buckets: BTreeMap<u8, RateLimit>,
}

impl ChatGate {
    /// A gate with no buckets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks one `ChatSend` from `sender` at `now_ms`; `is_member` says whether a slot is in the session. The relayed
    /// line carries `tick`.
    pub fn send(
        &mut self,
        sender: Sender,
        send: &ChatSend,
        is_member: impl Fn(u8) -> bool,
        tick: u32,
        now_ms: u64,
    ) -> Result<Relay, ChatRefusal> {
        self.admit(sender, now_ms)?;
        let audience = match (send.channel, send.to_slot) {
            (ChatChannel::All, _) => Audience::All,
            (ChatChannel::Team, _) => Audience::Team(sender.team),
            (ChatChannel::Whisper, Some(to)) if to != sender.slot && is_member(to) => {
                Audience::Whisper { from: sender.slot, to }
            }
            (ChatChannel::Whisper, _) => return Err(ChatRefusal::UnknownTarget),
        };
        let text = normalise(&send.text).ok_or(ChatRefusal::Empty)?;
        let message = ChatMessage { from_slot: sender.slot, channel: send.channel, text, tick };
        Ok(Relay { message, audience })
    }

    /// Checks one `QuickMessage` from `sender` at `now_ms`. It shares the chat bucket.
    pub fn quick(&mut self, sender: Sender, quick: &QuickMessage, now_ms: u64) -> Result<QuickRelay, ChatRefusal> {
        if !is_quick_id(quick.id) {
            return Err(ChatRefusal::UnknownQuick);
        }
        self.admit(sender, now_ms)?;
        Ok(QuickRelay { from: sender.slot, id: quick.id })
    }

    /// Drops the bucket of a slot that left, so a new member of the slot starts full.
    pub fn forget(&mut self, slot: u8) {
        self.buckets.remove(&slot);
    }

    fn admit(&mut self, sender: Sender, now_ms: u64) -> Result<(), ChatRefusal> {
        if sender.muted {
            return Err(ChatRefusal::Muted);
        }
        let bucket = self.buckets.entry(sender.slot).or_insert_with(|| RateLimit::new(CHAT_RATE, now_ms));
        if !bucket.allow(now_ms) {
            return Err(ChatRefusal::RateLimited);
        }
        Ok(())
    }
}
