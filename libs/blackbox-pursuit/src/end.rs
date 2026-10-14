//! How a pursuit ended, as the camera, the overlay and the bounty read it. Plain data; the caller decides the outcome
//! (the busted timer for `Busted`, the cool-down for `Evaded`).

/// How the chase ended. `None` while one runs, and before any chase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EndReason {
    /// No result: the chase runs, or it was stopped without one (the `pursuit off` command).
    #[default]
    None,
    /// The player was caught: the busted timer passed its timeout (`docs/specs/ai-pursuit.md` section 5).
    Busted,
    /// The player got away: the cool-down meter reached 1 while shown (section 6, status EVADED).
    Evaded,
}
