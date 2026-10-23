//! The message catalogue: every type number, its channel and format, the [`Message`] enum, and [`encode`] and
//! [`decode`] of a whole message (`type` u16 little-endian, `format` u8, body).

use super::admission::*;
use super::chat::*;
use super::codec::{DecodeError, EncodeError, Reader, Writer};
use super::events::*;
use super::lobby::*;
use super::wire::Wire;

/// Bytes of the message header: `type` u16 and `format` u8.
pub const HEADER_LEN: usize = 3;
/// The largest reliable message, header included: what the connection layer can carry in 16 fragments.
pub const MAX_MESSAGE_BYTES: usize = blackbox_net_core::conn::MAX_MESSAGE;

/// The reliable channel a message travels on (protocol.md section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Control = 0,
    Events = 1,
    Chat = 2,
    Bulk = 3,
}

impl Channel {
    /// The channel index of the connection layer.
    pub fn index(self) -> u8 {
        self as u8
    }
}

/// One row per message: variant (and body type), type number, channel, the newest format this build writes.
macro_rules! catalogue {
    ( $( $variant:ident = $type_id:literal, $channel:ident, $format:literal; )* ) => {
        /// Every reliable message of the protocol. The variant and its body type share a name.
        // The bodies differ in size (settings against a single bool); messages are few per second, so no boxing.
        #[allow(clippy::large_enum_variant)]
        #[derive(Debug, Clone, PartialEq)]
        pub enum Message {
            $( $variant($variant), )*
        }

        impl Message {
            /// The type number.
            pub fn type_id(&self) -> u16 {
                match self {
                    $( Self::$variant(_) => $type_id, )*
                }
            }

            /// The channel it travels on.
            pub fn channel(&self) -> Channel {
                match self {
                    $( Self::$variant(_) => Channel::$channel, )*
                }
            }

            /// The format this build writes for the type.
            pub fn format(&self) -> u8 {
                match self {
                    $( Self::$variant(_) => $format, )*
                }
            }

            /// The format this build writes for `type_id`, or `None` for a type outside the catalogue.
            pub fn known_format(type_id: u16) -> Option<u8> {
                match type_id {
                    $( $type_id => Some($format), )*
                    _ => None,
                }
            }

            fn put_body(&self, w: &mut Writer<'_>) {
                match self {
                    $( Self::$variant(body) => body.put(w), )*
                }
            }

            fn take_body(type_id: u16, r: &mut Reader<'_>) -> Result<Self, DecodeError> {
                match type_id {
                    $( $type_id => Ok(Self::$variant(<$variant as Wire>::take(r)?)), )*
                    other => Err(DecodeError::UnknownType(other)),
                }
            }
        }

        $(
            impl From<$variant> for Message {
                fn from(body: $variant) -> Self {
                    Self::$variant(body)
                }
            }
        )*
    };
}

catalogue! {
    // Admission, core namespace.
    Auth = 0x0001, Control, 1;
    PakeStart = 0x0002, Control, 1;
    PakeMsg = 0x0003, Control, 1;
    PakeConfirm = 0x0004, Control, 1;
    Join = 0x0005, Control, 1;
    JoinAccepted = 0x0006, Control, 1;
    JoinRejected = 0x0007, Control, 1;
    Kick = 0x0008, Control, 1;
    // Lobby and session, core namespace.
    LobbyState = 0x0010, Control, 1;
    LobbyDelta = 0x0011, Control, 1;
    SetReady = 0x0012, Control, 1;
    SelectLoadout = 0x0013, Control, 1;
    SetRole = 0x0014, Control, 1;
    UpdateSettings = 0x0015, Control, 1;
    StartCountdown = 0x0016, Control, 1;
    CancelCountdown = 0x0017, Control, 1;
    LoadWorld = 0x0018, Control, 1;
    LoadComplete = 0x0019, Control, 1;
    StartAt = 0x001A, Control, 1;
    Vote = 0x001B, Control, 1;
    VoteState = 0x001C, Control, 1;
    HostCommand = 0x001D, Control, 1;
    StreamingStall = 0x001E, Control, 1;
    ResultRequest = 0x001F, Control, 1;
    ResetRequest = 0x0020, Control, 1;
    BaselineLost = 0x0021, Control, 1;
    // Chat and bulk, core namespace.
    ChatSend = 0x0200, Chat, 1;
    ChatMessage = 0x0201, Chat, 1;
    SystemNotice = 0x0202, Chat, 1;
    QuickMessage = 0x0203, Chat, 1;
    QuickRelay = 0x0204, Chat, 1;
    BulkStart = 0x0300, Bulk, 1;
    BulkChunk = 0x0301, Bulk, 1;
    BulkEnd = 0x0302, Bulk, 1;
    // Gameplay events, game namespace (protocol.md's 0x01xx numbers plus 0x1000, ADR 0009).
    RaceStart = 0x1100, Events, 1;
    Checkpoint = 0x1101, Events, 1;
    LapComplete = 0x1102, Events, 1;
    RaceFinish = 0x1103, Events, 1;
    RaceResults = 0x1104, Events, 1;
    KnockedOut = 0x1105, Events, 1;
    TrapScore = 0x1106, Events, 1;
    TagPassed = 0x1107, Events, 1;
    Penalty = 0x1108, Events, 1;
    PursuitStart = 0x1110, Events, 1;
    PursuitEnd = 0x1111, Events, 1;
    HeatChange = 0x1112, Events, 1;
    Infraction = 0x1113, Events, 1;
    BustedProgress = 0x1114, Events, 1;
    RoadblockEvent = 0x1115, Events, 1;
    SpikeEvent = 0x1116, Events, 1;
    Collision = 0x1120, Events, 1;
    Damage = 0x1121, Events, 1;
    Puncture = 0x1122, Events, 1;
    Repair = 0x1123, Events, 1;
    Respawn = 0x1130, Events, 1;
    PhaseChange = 0x1131, Events, 1;
    EntityRemoved = 0x1132, Events, 1;
    PlayerJoined = 0x1140, Events, 1;
    PlayerLeft = 0x1141, Events, 1;
    RoleChanged = 0x1142, Events, 1;
    ClockEvent = 0x1150, Events, 1;
}

/// Writes `message` into `out` (cleared first). [`EncodeError::TooLarge`] when it would not fit the reliable channel;
/// `out` is then empty.
pub fn encode(message: &Message, out: &mut Vec<u8>) -> Result<(), EncodeError> {
    out.clear();
    out.extend_from_slice(&message.type_id().to_le_bytes());
    out.push(message.format());
    message.put_body(&mut Writer::new(out));
    if out.len() > MAX_MESSAGE_BYTES {
        out.clear();
        return Err(EncodeError::TooLarge);
    }
    Ok(())
}

/// Reads one whole message. The size limit is checked before anything else. A format newer than this build's has its
/// unknown trailing fields skipped (protocol.md section 8.2); at this build's format, trailing bytes are an error.
pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(DecodeError::TooLarge);
    }
    let mut r = Reader::new(bytes);
    let type_id = u16::from_le_bytes([r.u8()?, r.u8()?]);
    let format = r.u8()?;
    let known = Message::known_format(type_id).ok_or(DecodeError::UnknownType(type_id))?;
    if format < known {
        return Err(DecodeError::BadFormat { type_id, format });
    }
    let message = Message::take_body(type_id, &mut r)?;
    if format == known && !r.is_empty() {
        return Err(DecodeError::TrailingBytes);
    }
    Ok(message)
}

/// The type number of an encoded message, without decoding it.
pub fn peek_type(bytes: &[u8]) -> Option<u16> {
    match bytes {
        [low, high, ..] => Some(u16::from_le_bytes([*low, *high])),
        _ => None,
    }
}
