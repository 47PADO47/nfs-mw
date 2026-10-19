//! `Ui`: named themes a mod registers, each with a draw spec for every slot it overrides (catalogue: docs/modding/api-ui.md;
//! the table is `abi::BbUi`).
//!
//! A slot is a number the game assigns, so the mod passes it as a `u32`. A spec is a list of [`Prim`]s: panels, text and
//! images, in FEng units with the origin at the centre of the screen. The game builds them into components. No callback
//! crosses the boundary, so the wrapper keeps no closure and needs no host handle.

use super::ffi::ui::{self as host_ui, UiFns};
use super::{ModResult, UiThemeId};

/// A theme's tokens: five colours (`[r, g, b, a]`) and two shape values in FEng units. The game draws the Custom
/// components with them where the theme leaves a slot to them (docs/modding/api-ui.md).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    /// The main text and digits.
    pub primary: [u8; 4],
    /// The accent, such as a warning colour.
    pub secondary: [u8; 4],
    /// The fill of a panel.
    pub background: [u8; 4],
    /// Rules, tracks and the empty part of a gauge.
    pub border: [u8; 4],
    /// Labels and secondary text.
    pub muted: [u8; 4],
    /// The corner radius of a panel, in FEng units (never negative).
    pub radius: f32,
    /// The space between a panel's edge and its content, in FEng units (never negative).
    pub padding: f32,
}

/// A rectangle in FEng units: its centre at `x`, `y`, and its size `w` by `h` (never negative).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// The centre, x.
    pub x: f32,
    /// The centre, y.
    pub y: f32,
    /// The width.
    pub w: f32,
    /// The height.
    pub h: f32,
}

/// One primitive of a slot's draw spec. The names are copied when the spec is sent, so they need only last for the call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Prim<'a> {
    /// A filled rectangle in `colour` (`[r, g, b, a]`), with an optional texture over it.
    Panel { rect: Rect, colour: [u8; 4], texture: Option<&'a str> },
    /// A string in `font` (a resource name without extension; `""` for the default font), in `colour`, placed by
    /// `justify` (the FEng text flags: `H_CENTRE` 1, `H_RIGHT` 2, `V_CENTRE` 4, `V_BOTTOM` 8, `WRAP` 0x10).
    Text { rect: Rect, colour: [u8; 4], justify: u32, font: &'a str, text: &'a str },
    /// A texture (a resource name without extension) over the rectangle, showing the part `uv` names: `[u0, v0, u1, v1]`.
    Image { rect: Rect, uv: [f32; 4], texture: &'a str },
}

/// The `bb.ui` interface of the host, as a mod uses it. Obtained from `Ctx::ui`.
///
/// Each theme holds the slots it overrides. A slot the theme does not name keeps the game's own look.
pub struct Ui {
    fns: UiFns,
}

impl Ui {
    pub(crate) fn new(fns: UiFns) -> Self {
        Ui { fns }
    }

    /// Register a theme named `name` (plain, unique among this mod's themes), shown as `display`. Its id comes back.
    pub fn register_theme(&self, name: &str, display: &str) -> ModResult<UiThemeId> {
        host_ui::register_theme(&self.fns, name, display)
    }

    /// Give slot `slot` of `theme` the draw spec `prims`, drawn in order. A second call for the same slot replaces the
    /// first. A spec holds 1 to 64 primitives.
    pub fn set_slot(&self, theme: UiThemeId, slot: u32, prims: &[Prim<'_>]) -> ModResult<()> {
        host_ui::set_slot(&self.fns, theme, slot, prims)
    }

    /// Declare the tokens of `theme`: its colours, corner radius and padding. A theme that declares tokens also gets the
    /// Custom components for the slots it leaves to them. A second call replaces the first.
    pub fn set_tokens(&self, theme: UiThemeId, tokens: &Tokens) -> ModResult<()> {
        host_ui::set_tokens(&self.fns, theme, tokens)
    }

    /// Remove a theme this mod registered, with every slot it overrides.
    pub fn unregister_theme(&self, theme: UiThemeId) -> ModResult<()> {
        host_ui::unregister_theme(&self.fns, theme)
    }
}
