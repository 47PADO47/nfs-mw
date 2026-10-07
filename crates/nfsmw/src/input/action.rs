//! What the player can ask for, independent of the device that asks.

/// An abstract input. Axes carry a signed amount; buttons are 1 while held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Axis, -1..1: forward is positive.
    MoveForward,
    /// Axis, -1..1: right is positive.
    MoveRight,
    /// Axis, -1..1: up is positive.
    MoveUp,
    /// Free-look turn this frame, in mouse pixels: right is positive.
    LookX,
    /// Free-look turn this frame, in mouse pixels: down is positive.
    LookY,
    /// Orbit turn this frame, in mouse pixels: right is positive.
    OrbitX,
    /// Orbit turn this frame, in mouse pixels: down is positive.
    OrbitY,
    /// Zoom or speed change this frame, in scroll lines: away from the player is positive.
    Zoom,
    /// Button: go faster.
    Boost,
    /// Button: back out (release the mouse, then quit).
    Cancel,
}

impl Action {
    pub const ALL: [Action; 10] = [
        Action::MoveForward,
        Action::MoveRight,
        Action::MoveUp,
        Action::LookX,
        Action::LookY,
        Action::OrbitX,
        Action::OrbitY,
        Action::Zoom,
        Action::Boost,
        Action::Cancel,
    ];

    pub(super) fn index(self) -> usize {
        self as usize
    }

    /// Whether several devices adding up should still stay within -1..1.
    pub(super) fn is_bounded(self) -> bool {
        matches!(self, Action::MoveForward | Action::MoveRight | Action::MoveUp | Action::Boost | Action::Cancel)
    }
}
