//! Presenting the HUD: turning the FEng tree into something drawn.
//!
//! There is exactly one presenter, [`blackbox`], which makes meshes for `blackbox-render`'s UI layer. A
//! presenter takes the tree and the assets and writes into the frame's [`UiOutput`](crate::gui::UiOutput);
//! a Bevy UI presenter would replace `blackbox` without touching `HudState`, the binding or the game code
//! (see `docs/decisions/0002-ui-presentation.md`).

pub mod blackbox;

pub use blackbox::BlackboxPresenter;

/// Where the HUD is drawn, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub width: f32,
    pub height: f32,
    pub pixels_per_point: f32,
}

/// The logical size of an FEng screen.
pub const FENG_HEIGHT: f32 = 480.0;

impl Screen {
    /// Points per FEng unit: the 480-unit height fills the window height.
    pub fn scale(&self) -> f32 {
        self.height / FENG_HEIGHT
    }
}
