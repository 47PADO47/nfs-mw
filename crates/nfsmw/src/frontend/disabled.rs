//! How a greyed-out entry is drawn. Every screen that dims one uses these, so a disabled row, a disabled sheet title
//! and a disabled icon read the same wherever they appear.

/// The alpha a disabled text is capped at: the option rows, the infractions and the Blacklist pages.
pub const TEXT_ALPHA: u8 = 110;
/// The alpha a disabled icon is capped at (docs/specs/frontend-menus.md, section 2.1).
pub const ICON_ALPHA: u8 = 150;
