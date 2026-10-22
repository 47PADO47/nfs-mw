//! Chat (protocol.md section 7.7, channel `chat`) and bulk transfers (section 7.8, channel `bulk`).

use super::common::{ensure, is_slot};
use super::macros::{wire_enum, wire_struct};
use super::text::ChatText;
use super::wire::{Bytes, List};

/// Most bytes of one bulk chunk.
pub const MAX_BULK_CHUNK: usize = 1024;
/// Most bytes of one bulk transfer (`security-gameplay.md` section 1.2).
pub const MAX_BULK_TRANSFER: u32 = 4 * 1024 * 1024;
/// Most arguments of a system notice.
pub const MAX_NOTICE_ARGS: usize = 4;

wire_enum! {
    pub enum ChatChannel {
        All = 0,
        Team = 1,
        /// To one slot.
        Whisper = 2,
    }
}

wire_struct! {
    /// A player's chat line. `to_slot` is set exactly for a whisper.
    pub struct ChatSend {
        pub channel: ChatChannel,
        pub to_slot: Option<u8>,
        pub text: ChatText,
    }
    check(send) {
        let whisper = send.channel == ChatChannel::Whisper;
        ensure(whisper == send.to_slot.is_some() && send.to_slot.is_none_or(is_slot))
    }
}

wire_struct! {
    /// A chat line as the server relays it.
    pub struct ChatMessage {
        pub from_slot: u8,
        pub channel: ChatChannel,
        pub text: ChatText,
        pub tick: u32,
    }
    check(message) { ensure(is_slot(message.from_slot)) }
}

wire_enum! {
    pub enum NoticeCode {
        Join = 0,
        Leave = 1,
        Kick = 2,
        Mute = 3,
        Vote = 4,
        Result = 5,
    }
}

wire_struct! {
    /// A system line: a code and its numeric arguments (slots, counts), drawn by the client in its own language.
    pub struct SystemNotice {
        pub code: NoticeCode,
        pub args: List<u32, MAX_NOTICE_ARGS>,
    }
}

wire_struct! {
    /// A preset line (MP-44).
    pub struct QuickMessage {
        pub id: u16,
    }
}

wire_enum! {
    pub enum BulkKind {
        SessionData = 0,
        ModManifest = 1,
        ReplayChunk = 2,
    }
}

wire_struct! {
    /// A transfer begins: its length and the SHA-256 the receiver checks before use.
    pub struct BulkStart {
        pub kind: BulkKind,
        pub id: u16,
        /// At most [`MAX_BULK_TRANSFER`].
        pub length: u32,
        pub hash: [u8; 32],
    }
    check(start) { ensure(start.length <= MAX_BULK_TRANSFER) }
}

wire_struct! {
    /// One chunk of a transfer, at most [`MAX_BULK_CHUNK`] bytes, ending inside the transfer limit.
    pub struct BulkChunk {
        pub id: u16,
        pub offset: u32,
        pub bytes: Bytes<MAX_BULK_CHUNK>,
    }
    check(chunk) {
        let end = u64::from(chunk.offset) + chunk.bytes.len() as u64;
        ensure(end <= u64::from(MAX_BULK_TRANSFER))
    }
}

wire_struct! {
    pub struct BulkEnd {
        pub id: u16,
    }
}
