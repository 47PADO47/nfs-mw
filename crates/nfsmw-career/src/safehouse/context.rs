//! The front-end context word (`FEDatabase +0x12C`), the bits the safe house flow uses
//! (`docs/specs/career-flow.md` section 1, `docs/specs/career-safehouse.md` section 1).

/// The front-end context word. Unknown bits are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrontEndContext(pub u32);

impl FrontEndContext {
    /// Bit 0: career.
    pub const CAREER: FrontEndContext = FrontEndContext(0x1);
    /// Bit 3: online.
    pub const ONLINE: FrontEndContext = FrontEndContext(0x8);
    /// Bit 5: customize (the custom shop and My Cars).
    pub const CUSTOMIZE: FrontEndContext = FrontEndContext(0x20);
    /// Bit 6: LAN.
    pub const LAN: FrontEndContext = FrontEndContext(0x40);
    /// Bit 8: career manager (the profile screens).
    pub const CAREER_MANAGER: FrontEndContext = FrontEndContext(0x100);
    /// Bit 9: the Rap Sheet.
    pub const RAP_SHEET: FrontEndContext = FrontEndContext(0x200);
    /// Bit 15: car lot.
    pub const CAR_LOT: FrontEndContext = FrontEndContext(0x8000);
    /// Bit 17: a Blacklist sequence is on screen.
    pub const BLACKLIST_SEQUENCE: FrontEndContext = FrontEndContext(0x20000);
    /// Bit 18: game won.
    pub const GAME_WON: FrontEndContext = FrontEndContext(0x40000);

    /// True when every bit of `other` is set.
    pub const fn contains(self, other: FrontEndContext) -> bool {
        self.0 & other.0 == other.0
    }

    /// Sets the bits of `other`.
    pub fn insert(&mut self, other: FrontEndContext) {
        self.0 |= other.0;
    }

    /// Clears the bits of `other`.
    pub fn remove(&mut self, other: FrontEndContext) {
        self.0 &= !other.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_match_the_documented_values() {
        assert_eq!(FrontEndContext::CAREER.0, 1 << 0);
        assert_eq!(FrontEndContext::ONLINE.0, 1 << 3);
        assert_eq!(FrontEndContext::CUSTOMIZE.0, 1 << 5);
        assert_eq!(FrontEndContext::LAN.0, 1 << 6);
        assert_eq!(FrontEndContext::CAREER_MANAGER.0, 1 << 8);
        assert_eq!(FrontEndContext::RAP_SHEET.0, 1 << 9);
        assert_eq!(FrontEndContext::CAR_LOT.0, 1 << 15);
        assert_eq!(FrontEndContext::BLACKLIST_SEQUENCE.0, 1 << 17);
        assert_eq!(FrontEndContext::GAME_WON.0, 1 << 18);
    }

    #[test]
    fn insert_and_remove_touch_only_their_bits() {
        let mut c = FrontEndContext::CAREER;
        c.insert(FrontEndContext::BLACKLIST_SEQUENCE);
        assert!(c.contains(FrontEndContext::CAREER) && c.contains(FrontEndContext::BLACKLIST_SEQUENCE));
        c.remove(FrontEndContext::BLACKLIST_SEQUENCE);
        assert_eq!(c, FrontEndContext::CAREER);
    }
}
