//! Session settings rules (MP-25, MP-33, MP-34): defaults, validation beyond the wire ranges, a host's patch with its
//! permission and phase checks, and a flat key and value form a config file maps onto.
//!
//! The struct itself is [`crate::message::Settings`] (protocol-messages.md 7.3); its decoder already refuses values
//! outside the wire ranges. This module adds the session rules:
//!
//! - [`defaults`] and [`dedicated_defaults`]: 8 racers for a LAN listen server, 16 for a dedicated one.
//! - [`validate`]: the wire ranges plus the cross-field rules (laps for lap modes, police for pursuits, modes that are
//!   not built, a custom mode without a mod), naming the [`Field`] that broke.
//! - [`apply_patch`]: a `SettingsPatch` from a player, checked against the sender's [`crate::roles::Rank`], the phase
//!   (gameplay fields are locked from loading to the end of the race) and the current head counts.
//! - [`Field`], [`to_pairs`], [`set`] and [`from_pairs`]: every editable field as a stable lower-case key and a text
//!   value, so the server crate can read a TOML table without this crate depending on a TOML parser. The required mods
//!   are not a pair: they follow the server's own mod list.
//!
//! The room password itself never lives in the settings: the struct carries `password_required` only, and the server
//! holds the password in memory (security.md 4.6).

mod pairs;
mod rules;

#[cfg(test)]
mod tests;

pub use pairs::{Field, from_pairs, set, to_pairs};
pub use rules::{PatchContext, SettingsError, apply_patch, has_laps, validate};

use crate::message::{ServerName, Settings};

/// Racers of a LAN session by default (features.md MP-24).
pub const LAN_PLAYERS: u8 = 8;
/// Racers of a dedicated server by default: the design limit (ADR 0006).
pub const DEDICATED_PLAYERS: u8 = 16;
/// Spectators of a dedicated server by default.
pub const DEDICATED_SPECTATORS: u8 = 8;

/// The LAN defaults for a listen server called `name`: [`LAN_PLAYERS`] racers, free roam, pursuits on, collisions on,
/// nitrous on, visual damage, open in free roam, no password, LAN only.
pub fn defaults(name: ServerName) -> Settings {
    Settings::lan(name)
}

/// The defaults of a dedicated server called `name`: as [`defaults`] with [`DEDICATED_PLAYERS`] racers and
/// [`DEDICATED_SPECTATORS`] spectators.
pub fn dedicated_defaults(name: ServerName) -> Settings {
    Settings { max_players: DEDICATED_PLAYERS, max_spectators: DEDICATED_SPECTATORS, ..Settings::lan(name) }
}
