//! The input events a component can handle. Deliberately small: a pad button or a pointer action.
//! A host translates its device events into these before they reach the tree.

/// A pad button in navigation terms (not a physical button name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadButton {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
}

/// What the pointer did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Move,
    Press,
    Release,
}

/// One input event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputEvent {
    /// A pad button went down (`pressed`) or up.
    Pad { button: PadButton, pressed: bool },
    /// The pointer moved or clicked at `(x, y)` in screen units (origin centre, see `Rect`).
    Pointer { x: f32, y: f32, kind: PointerKind },
}
