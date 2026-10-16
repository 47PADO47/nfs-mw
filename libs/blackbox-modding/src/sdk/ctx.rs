//! `Ctx`: the mod's view of the host, handed to every callback.

use super::ffi::{self, HostTable};
use super::{Interface, ModResult};
use crate::abi::{AbiVersion, BB_LOG_DEBUG, BB_LOG_ERROR, BB_LOG_INFO, BB_LOG_TRACE, BB_LOG_WARN};

/// The longest message the host takes in one line (ADR 0005 section 4). Longer text is cut at a character boundary.
pub const MAX_MESSAGE_BYTES: usize = 1024;

/// Log severity. Maps to the host's `BB_LOG_*` levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Level {
    /// Something failed.
    Error = BB_LOG_ERROR,
    /// Something is wrong but the game goes on.
    Warn = BB_LOG_WARN,
    /// Normal progress.
    Info = BB_LOG_INFO,
    /// Details for a developer.
    Debug = BB_LOG_DEBUG,
    /// Very detailed output.
    Trace = BB_LOG_TRACE,
}

/// The host as a mod sees it inside a callback. A `Ctx` cannot be stored or built outside the SDK, so it is
/// valid only for the callback that received it.
pub struct Ctx {
    host: HostTable,
}

impl Ctx {
    /// Wrap the host table of the current mod.
    pub(crate) fn new(host: HostTable) -> Self {
        Ctx { host }
    }

    /// The ABI version the host reports (the one the handshake accepted).
    pub fn abi_version(&self) -> AbiVersion {
        self.host.version()
    }

    /// Write one line to the game log, cut to [`MAX_MESSAGE_BYTES`].
    pub fn log(&self, level: Level, msg: &str) {
        ffi::log(self.host, level as u32, clip(msg, MAX_MESSAGE_BYTES));
    }

    /// The game id the host reports, for example a short name of the game.
    pub fn game_id(&self) -> ModResult<String> {
        ffi::game_id(self.host)
    }

    /// The game version string the host reports.
    pub fn game_version(&self) -> ModResult<String> {
        ffi::game_version(self.host)
    }

    /// Ask the host for an interface by id. `None` when the host does not offer it, offers another major, or offers
    /// a minor below `min_minor`. A mod must cope with `None` for an optional interface.
    pub fn query_interface(&self, id: &str, major: u16, min_minor: u16) -> Option<Interface> {
        ffi::query_interface(self.host, id, major, min_minor)
    }
}

/// Cut `text` to at most `max` bytes, at a character boundary.
fn clip(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_unchanged() {
        assert_eq!(clip("abc", 8), "abc");
    }

    #[test]
    fn long_text_is_cut_at_a_character_boundary() {
        // Each "é" is two bytes, so a cut at byte 3 must back off to byte 2.
        assert_eq!(clip("ééé", 3), "é");
        assert_eq!(clip("ééé", 4), "éé");
    }

    #[test]
    fn levels_match_the_host_constants() {
        assert_eq!(Level::Error as u32, BB_LOG_ERROR);
        assert_eq!(Level::Trace as u32, BB_LOG_TRACE);
    }
}
