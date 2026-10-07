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
    /// Button: back out (close the console, release the mouse, then quit).
    Cancel,
    /// Button: open or close the developer console.
    Console,
}

impl Action {
    pub const ALL: [Action; 11] = [
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
        Action::Console,
    ];

    /// Actions that still work while the UI has the keyboard.
    pub(super) fn works_in_ui(self) -> bool {
        matches!(self, Action::Cancel | Action::Console)
    }

    pub(super) fn index(self) -> usize {
        self as usize
    }

    /// Whether several devices adding up should still stay within -1..1.
    pub(super) fn is_bounded(self) -> bool {
        matches!(
            self,
            Action::MoveForward | Action::MoveRight | Action::MoveUp | Action::Boost | Action::Cancel | Action::Console
        )
    }
}
