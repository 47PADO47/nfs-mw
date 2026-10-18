//! `bb.ui` 1.0: a named theme a mod registers, and a declarative draw spec for each slot it overrides (catalogue:
//! docs/modding/api-ui.md; design: docs/specs/ui-themes.md section 12).
//!
//! The C ABI cannot carry a component, so a slot is described by a flat list of primitives. The host builds each list
//! into components from its own base components. Three primitive kinds exist, and the set stays small on purpose.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbStatus, BbStr};

/// A filled rectangle in the slot's colour, with an optional texture over it.
pub const BB_UI_PANEL: u32 = 0;
/// A string in a font, placed in the rectangle by its justification flags.
pub const BB_UI_TEXT: u32 = 1;
/// A texture over the rectangle, showing the part of it that `uv` names.
pub const BB_UI_IMAGE: u32 = 2;

/// One primitive of a slot's draw spec, read by `set_slot`. The rectangle is in FEng units with the origin at the
/// centre of the screen. The strings are valid for the call that carries the primitive. A kind uses only the fields
/// its comment names; the others are zero.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbUiPrim {
    /// Bytes the mod filled in.
    pub size: u32,
    /// `BB_UI_PANEL`, `BB_UI_TEXT` or `BB_UI_IMAGE`.
    pub kind: u32,
    /// The rectangle's centre, x.
    pub x: f32,
    /// The rectangle's centre, y.
    pub y: f32,
    /// The rectangle's width.
    pub w: f32,
    /// The rectangle's height.
    pub h: f32,
    /// Panel: the fill. Text: the colour of the string. Image: unused.
    pub colour: [u8; 4],
    /// Text: the justification flags, the bits of the FEng text style (`H_CENTRE` 1, `H_RIGHT` 2, `V_CENTRE` 4,
    /// `V_BOTTOM` 8, `WRAP` 0x10). Other kinds: zero.
    pub justify: u32,
    /// Image: the part of the texture to show, `[u0, v0, u1, v1]`. Other kinds: zero.
    pub uv: [f32; 4],
    /// Text: the font's resource name, without extension (`FONT_MW_BODY`). Empty means the default font.
    pub font: BbStr,
    /// Panel: an optional texture name, empty for none. Image: the texture's resource name, without extension.
    pub texture: BbStr,
    /// Text: the string shown.
    pub text: BbStr,
}

/// `register_theme`: a theme named `name` (plain, unique among the mod's themes) shown as `display`. Its handle goes to
/// `out_id`.
pub type BbUiRegisterThemeFn =
    unsafe extern "C" fn(ctx: *mut c_void, name: BbStr, display: BbStr, out_id: *mut u64) -> BbStatus;
/// `set_slot`: the draw spec of slot `slot` in theme `theme`. `prims` holds `count` primitives, drawn in order. A second
/// call for the same slot replaces the first.
pub type BbUiSetSlotFn =
    unsafe extern "C" fn(ctx: *mut c_void, theme: u64, slot: u32, prims: *const BbUiPrim, count: u64) -> BbStatus;
/// `unregister_theme`: remove a theme and every slot it overrides.
pub type BbUiUnregisterThemeFn = unsafe extern "C" fn(ctx: *mut c_void, theme: u64) -> BbStatus;

/// The `bb.ui` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbUi {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Register a theme.
    pub register_theme: Option<BbUiRegisterThemeFn>,
    /// Describe one slot of a theme.
    pub set_slot: Option<BbUiSetSlotFn>,
    /// Remove a theme.
    pub unregister_theme: Option<BbUiUnregisterThemeFn>,
}
