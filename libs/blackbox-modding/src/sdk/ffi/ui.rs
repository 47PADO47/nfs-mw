//! `bb.ui` on the mod side: read the table, register a theme, and send a slot's draw spec as the C structs the host reads.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{
    BB_UI_IMAGE, BB_UI_PANEL, BB_UI_TEXT, BbStr, BbUi, BbUiPrim, BbUiRegisterThemeFn, BbUiSetSlotFn,
    BbUiSetTokensFn, BbUiTokens, BbUiUnregisterThemeFn,
};
use crate::sdk::ui::{Prim, Rect, Tokens};
use crate::sdk::{Error, Interface, ModResult, UiThemeId};

/// The function pointers of one `bb.ui` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct UiFns {
    ctx: *mut c_void,
    register_theme: Option<BbUiRegisterThemeFn>,
    set_slot: Option<BbUiSetSlotFn>,
    unregister_theme: Option<BbUiUnregisterThemeFn>,
    set_tokens: Option<BbUiSetTokensFn>,
}

/// Read the `bb.ui` table the host offered.
pub(crate) fn read(iface: Interface) -> UiFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        UiFns {
            ctx: read_or(&iface, offset_of!(BbUi, ctx), null_mut()),
            register_theme: read_or(&iface, offset_of!(BbUi, register_theme), None),
            set_slot: read_or(&iface, offset_of!(BbUi, set_slot), None),
            unregister_theme: read_or(&iface, offset_of!(BbUi, unregister_theme), None),
            set_tokens: read_or(&iface, offset_of!(BbUi, set_tokens), None),
        }
    }
}

/// Register a theme; its handle.
pub(crate) fn register_theme(fns: &UiFns, name: &str, display: &str) -> ModResult<UiThemeId> {
    let register = fns.register_theme.ok_or(Error::Unsupported)?;
    let mut id = 0_u64;
    // SAFETY: the strings are valid for the call, `id` is valid for one write, and the table is valid while the mod is
    // loaded.
    status_result(unsafe { register(fns.ctx, BbStr::new(name), BbStr::new(display), &mut id) })?;
    Ok(UiThemeId::from_raw(id))
}

/// Give slot `slot` of `theme` the draw spec `prims`.
pub(crate) fn set_slot(fns: &UiFns, theme: UiThemeId, slot: u32, prims: &[Prim<'_>]) -> ModResult<()> {
    let set = fns.set_slot.ok_or(Error::Unsupported)?;
    let abi: Vec<BbUiPrim> = prims.iter().map(to_abi).collect();
    // SAFETY: `abi` holds `len` primitives whose strings borrow from `prims`, both valid for this call.
    status_result(unsafe { set(fns.ctx, theme.raw(), slot, abi.as_ptr(), abi.len() as u64) })
}

/// Declare the tokens of `theme`. A host older than `bb.ui` 1.1 has no `set_tokens`, and the call is refused as
/// unsupported.
pub(crate) fn set_tokens(fns: &UiFns, theme: UiThemeId, tokens: &Tokens) -> ModResult<()> {
    let set = fns.set_tokens.ok_or(Error::Unsupported)?;
    let abi = BbUiTokens {
        primary: tokens.primary,
        secondary: tokens.secondary,
        background: tokens.background,
        border: tokens.border,
        muted: tokens.muted,
        radius: tokens.radius,
        padding: tokens.padding,
    };
    // SAFETY: `abi` is valid for one read for this call, and the table is valid while the mod is loaded.
    status_result(unsafe { set(fns.ctx, theme.raw(), &abi) })
}

/// Remove a theme this mod registered.
pub(crate) fn unregister_theme(fns: &UiFns, theme: UiThemeId) -> ModResult<()> {
    let unregister = fns.unregister_theme.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, theme.raw()) })
}

/// The C struct of one primitive, with every field the kind does not use zeroed.
fn to_abi(prim: &Prim<'_>) -> BbUiPrim {
    let base = BbUiPrim {
        size: size_of::<BbUiPrim>() as u32,
        kind: BB_UI_PANEL,
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        colour: [0; 4],
        justify: 0,
        uv: [0.0; 4],
        font: BbStr::EMPTY,
        texture: BbStr::EMPTY,
        text: BbStr::EMPTY,
    };
    match *prim {
        Prim::Panel { rect, colour, texture } => BbUiPrim {
            kind: BB_UI_PANEL,
            colour,
            texture: texture.map_or(BbStr::EMPTY, BbStr::new),
            ..with_rect(base, rect)
        },
        Prim::Text { rect, colour, justify, font, text } => BbUiPrim {
            kind: BB_UI_TEXT,
            colour,
            justify,
            font: BbStr::new(font),
            text: BbStr::new(text),
            ..with_rect(base, rect)
        },
        Prim::Image { rect, uv, texture } => {
            BbUiPrim { kind: BB_UI_IMAGE, uv, texture: BbStr::new(texture), ..with_rect(base, rect) }
        }
    }
}

/// `base` with the rectangle `rect` in its position and size fields.
fn with_rect(base: BbUiPrim, rect: Rect) -> BbUiPrim {
    BbUiPrim { x: rect.x, y: rect.y, w: rect.w, h: rect.h, ..base }
}
