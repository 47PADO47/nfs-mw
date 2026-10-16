//! The typed `nfsmw.*` events (catalogue: docs/modding/api-nfsmw.md, "Events").
//!
//! The generic SDK hands an event it does not describe to the mod as `Event::Unknown` (blackbox-modding), without the
//! payload bytes. [`MwEvent::decode`] takes the kind, the frame and the payload bytes, and builds the typed event from them.
//! It is pure: it reads the bytes with bounds checks and never trusts a size word past the bytes it was given.

use blackbox_modding::abi::has_field;

use crate::abi::{EndReason, Infraction};

/// One `nfsmw.*` event, decoded. An event of another kind, or a payload too short for its kind, is [`MwEvent::Unknown`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum MwEvent {
    /// A chase began.
    PursuitStarted {
        /// Frame number.
        frame: u64,
        /// Heat level the chase started at, 1 to 10.
        level: u32,
        /// Cops in the first wave.
        cops: u32,
        /// Heat when it began.
        heat: f32,
    },
    /// A chase ended.
    PursuitEnded {
        /// Frame number.
        frame: u64,
        /// Why it ended.
        reason: EndReason,
        /// Bounty held at the end.
        bounty: u64,
        /// Seconds the chase lasted.
        duration: f32,
    },
    /// The heat moved.
    HeatChanged {
        /// Frame number.
        frame: u64,
        /// Level after the change, 1 to 10.
        level: u32,
        /// Heat before.
        old_heat: f32,
        /// Heat after.
        new_heat: f32,
    },
    /// The player was busted.
    Busted {
        /// Frame number.
        frame: u64,
        /// Heat level at the bust, 1 to 10.
        level: u32,
        /// Bounty at the bust.
        bounty: u64,
    },
    /// The player escaped the chase.
    Escaped {
        /// Frame number.
        frame: u64,
        /// Heat level at the escape, 1 to 10.
        level: u32,
        /// Seconds the chase lasted.
        duration: f32,
    },
    /// An infraction was recorded.
    InfractionRecorded {
        /// Frame number.
        frame: u64,
        /// Which infraction.
        infraction: Infraction,
        /// Fine, in cash.
        fine: u32,
    },
    /// Not an `nfsmw.*` event this SDK describes: another kind, or a payload too short for its kind.
    Unknown {
        /// The kind as the host sent it.
        kind: u32,
        /// Frame number.
        frame: u64,
    },
}

impl MwEvent {
    /// Decode one event from its kind, frame and payload bytes. The payload's own size word says how many of its bytes the
    /// host filled in; a field past that, or past the given bytes, makes the event `Unknown`.
    pub fn decode(kind: u32, frame: u64, payload: &[u8]) -> MwEvent {
        decode_known(kind, frame, payload).unwrap_or(MwEvent::Unknown { kind, frame })
    }
}

/// The typed event of a known kind, or `None` when the kind is unknown or the payload does not cover its fields.
fn decode_known(kind: u32, frame: u64, payload: &[u8]) -> Option<MwEvent> {
    match kind {
        crate::abi::MW_EVENT_PURSUIT_STARTED => {
            if !covers(payload, 16) {
                return None;
            }
            Some(MwEvent::PursuitStarted {
                frame,
                level: u32_at(payload, 4)?,
                cops: u32_at(payload, 8)?,
                heat: f32_at(payload, 12)?,
            })
        }
        crate::abi::MW_EVENT_PURSUIT_ENDED => {
            if !covers(payload, 24) {
                return None;
            }
            Some(MwEvent::PursuitEnded {
                frame,
                reason: EndReason::from_raw(u32_at(payload, 4)?)?,
                bounty: u64_at(payload, 8)?,
                duration: f32_at(payload, 16)?,
            })
        }
        crate::abi::MW_EVENT_HEAT_CHANGED => {
            if !covers(payload, 16) {
                return None;
            }
            Some(MwEvent::HeatChanged {
                frame,
                level: u32_at(payload, 4)?,
                old_heat: f32_at(payload, 8)?,
                new_heat: f32_at(payload, 12)?,
            })
        }
        crate::abi::MW_EVENT_BUSTED => {
            if !covers(payload, 16) {
                return None;
            }
            Some(MwEvent::Busted { frame, level: u32_at(payload, 4)?, bounty: u64_at(payload, 8)? })
        }
        crate::abi::MW_EVENT_ESCAPED => {
            if !covers(payload, 12) {
                return None;
            }
            Some(MwEvent::Escaped { frame, level: u32_at(payload, 4)?, duration: f32_at(payload, 8)? })
        }
        crate::abi::MW_EVENT_INFRACTION_RECORDED => {
            if !covers(payload, 12) {
                return None;
            }
            Some(MwEvent::InfractionRecorded {
                frame,
                infraction: Infraction::from_raw(u32_at(payload, 4)?)?,
                fine: u32_at(payload, 8)?,
            })
        }
        _ => None,
    }
}

/// Whether the payload's size word covers a field that ends at `end`, and the bytes given reach that far too.
fn covers(payload: &[u8], end: usize) -> bool {
    match u32_at(payload, 0) {
        Some(size) => payload.len() >= end && has_field(size, end),
        None => false,
    }
}

/// A `u32` at byte `at`, in native byte order (the layout of the `repr(C)` payload).
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let word: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_ne_bytes(word))
}

/// A `u64` at byte `at`, in native byte order.
fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    let word: [u8; 8] = bytes.get(at..at + 8)?.try_into().ok()?;
    Some(u64::from_ne_bytes(word))
}

/// An `f32` at byte `at`, in native byte order.
fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    u32_at(bytes, at).map(f32::from_bits)
}
