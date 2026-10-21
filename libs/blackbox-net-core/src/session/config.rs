//! Timing of a session.

/// Timing of a session. All times are in the units named on each field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionConfig {
    /// Milliseconds after `host_loaded` before peers that have not loaded are dropped. Default 120 000.
    pub load_timeout_ms: u64,
    /// Milliseconds of intro. Zero skips the intro. Default 0.
    pub intro_ms: u64,
    /// Ticks of countdown before the race starts. Default 180.
    pub countdown_ticks: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { load_timeout_ms: 120_000, intro_ms: 0, countdown_ticks: 180 }
    }
}
