//! The event a mod receives in `Mod::event`.

/// One event delivered to `Mod::event`. Only the header is decoded here: the kind and the frame number. The
/// per-kind payload is read by typed accessors that come with the event table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    kind: u32,
    frame: u64,
}

impl Event {
    /// Build an event from its header fields.
    pub(crate) const fn new(kind: u32, frame: u64) -> Self {
        Event { kind, frame }
    }

    /// The event kind id. Kinds are never reused, and a mod ignores kinds it does not know.
    pub const fn kind(&self) -> u32 {
        self.kind
    }

    /// The frame number at delivery.
    pub const fn frame(&self) -> u64 {
        self.frame
    }
}
