//! Overridable slot keys. A slot is a number the host assigns: the crate does not know what each slot
//! is. The host that owns the UI (the game, or a mod) defines its own catalogue and converts each entry to a
//! `SlotId`, so names such as "minimap" or "main menu" stay out of `libs/`.

/// An opaque, stable key for one overridable slot. Equal ids are the same slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlotId(u32);

impl SlotId {
    /// Wraps a host-assigned number. The host keeps the numbering stable across builds, since a theme or mod
    /// refers to slots by number.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// The host-assigned number.
    pub const fn raw(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SlotId;

    #[test]
    fn same_number_is_same_slot() {
        assert_eq!(SlotId::new(4), SlotId::new(4));
        assert_ne!(SlotId::new(4), SlotId::new(5));
        assert_eq!(SlotId::new(4).raw(), 4);
    }
}
