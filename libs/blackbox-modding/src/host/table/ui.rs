//! `bb.ui` on the host side: the backend the game implements, and the trampolines behind `BbUi`.
//!
//! The mod describes each slot with a flat list of primitives. This layer checks the shape of the list (sizes, finite
//! numbers, known kinds and justification bits) and hands the backend owned values. The game decides what the slot
//! means; a slot id it does not know is refused there.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, name_of, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BB_UI_IMAGE, BB_UI_PANEL, BB_UI_TEXT, BbIface, BbStatus, BbStr, BbUi, BbUiPrim, IFACE_UI,
    IFACE_UI_VERSION, MAX_MESSAGE_BYTES, MAX_NAME_BYTES,
};

/// The most primitives one slot may hold. A longer list is refused.
pub const MAX_PRIMITIVES: usize = 64;

/// The bits a text's justification may set: `H_CENTRE`, `H_RIGHT`, `V_CENTRE`, `V_BOTTOM` and `WRAP`.
const JUSTIFY_BITS: u32 = 0x1F;

/// A rectangle in FEng units, its centre at `x`, `y`, and its size `w` by `h`. Its size is never negative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One primitive of a slot's draw spec, as the host reads it. The names are copied out of the call.
#[derive(Debug, Clone, PartialEq)]
pub enum UiPrim {
    /// A filled rectangle, with an optional texture over it.
    Panel { rect: UiRect, colour: [u8; 4], texture: Option<String> },
    /// A string. An empty `font` means the default font.
    Text { rect: UiRect, colour: [u8; 4], justify: u32, font: String, text: String },
    /// A texture over the rectangle, showing the part of it that `uv` names (`[u0, v0, u1, v1]`).
    Image { rect: UiRect, uv: [f32; 4], texture: String },
}

/// A theme as the mod names it. The strings are valid for the call that registers the theme.
#[derive(Debug, Clone, PartialEq)]
pub struct UiThemeSpec<'a> {
    /// The theme's name, unique among the mod's themes. Its id in the game is derived from the mod id and this name.
    pub name: &'a str,
    /// The text the theme shows in the options.
    pub display: &'a str,
}

/// What the game implements for `bb.ui`. `owner` is the id of the mod that made the call.
pub trait UiBackend {
    /// Add a theme. Returns its handle.
    fn register_theme(&self, owner: &str, theme: &UiThemeSpec<'_>) -> BackendResult<u64>;
    /// Give slot `slot` of theme `theme` its draw spec. A second call for the same slot replaces the first.
    fn set_slot(&self, owner: &str, theme: u64, slot: u32, prims: &[UiPrim]) -> BackendResult<()>;
    /// Remove a theme and the slots it overrides.
    fn unregister_theme(&self, owner: &str, theme: u64) -> BackendResult<()>;
}

/// Build the `bb.ui` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn UiBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbUi {
        head: BbIface { size: size_of::<BbUi>() as u32, major: IFACE_UI_VERSION.0, minor: IFACE_UI_VERSION.1 },
        ctx: Slot::ctx(&slot),
        register_theme: Some(register_theme),
        set_slot: Some(set_slot),
        unregister_theme: Some(unregister_theme),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.unregister_theme(owner, handle);
        })
    });
    Attached::new(IFACE_UI, IFACE_UI_VERSION, table, release)
}

/// A number the mod sends must be finite.
fn finite(value: f32) -> Result<f32, BbStatus> {
    if !value.is_finite() {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// A size the mod sends: finite and not negative.
fn size(value: f32) -> Result<f32, BbStatus> {
    let value = finite(value)?;
    if value < 0.0 {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// A name the mod may leave empty to mean none: `None` for an empty string, else the name.
///
/// # Safety
/// As for [`name_of`]: `name` must be valid for the call.
unsafe fn optional_name(name: BbStr) -> Result<Option<String>, BbStatus> {
    if name.len == 0 {
        return Ok(None);
    }
    // SAFETY: forwarded to the caller's guarantee.
    Ok(Some(unsafe { name_of(name) }?.to_owned()))
}

/// Read one primitive the mod wrote. Its `size` must cover the whole struct; the kind decides which fields count.
///
/// # Safety
/// `prim` must be valid for reads of the bytes its own `size` field declares, and its strings valid for the call.
unsafe fn read_prim(prim: *const BbUiPrim) -> Result<UiPrim, BbStatus> {
    if prim.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the primitive's size word is its first field (see `# Safety`).
    let declared = unsafe { prim.cast::<u32>().read() };
    if (declared as usize) < size_of::<BbUiPrim>() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the size covers the struct, so the read is in bounds (see `# Safety`).
    let prim = unsafe { prim.read() };
    let rect = UiRect { x: finite(prim.x)?, y: finite(prim.y)?, w: size(prim.w)?, h: size(prim.h)? };
    match prim.kind {
        BB_UI_PANEL => {
            // SAFETY: the texture is the mod's, valid for the call (see `# Safety`).
            let texture = unsafe { optional_name(prim.texture) }?;
            Ok(UiPrim::Panel { rect, colour: prim.colour, texture })
        }
        BB_UI_TEXT => {
            if prim.justify & !JUSTIFY_BITS != 0 {
                return Err(BB_ERR_ARG);
            }
            // SAFETY: the mod's strings are valid for the call (see `# Safety`).
            let font = unsafe { name_of(prim.font) }?.to_owned();
            // SAFETY: as above.
            let text = unsafe { text_within(prim.text, MAX_MESSAGE_BYTES) }?.to_owned();
            Ok(UiPrim::Text { rect, colour: prim.colour, justify: prim.justify, font, text })
        }
        BB_UI_IMAGE => {
            let uv = [finite(prim.uv[0])?, finite(prim.uv[1])?, finite(prim.uv[2])?, finite(prim.uv[3])?];
            if prim.texture.len == 0 {
                return Err(BB_ERR_ARG);
            }
            // SAFETY: as above.
            let texture = unsafe { name_of(prim.texture) }?.to_owned();
            Ok(UiPrim::Image { rect, uv, texture })
        }
        _ => Err(BB_ERR_ARG),
    }
}

/// Read the `count` primitives at `prims`, in order.
///
/// # Safety
/// `prims` must be valid for reads of `count` primitives, and their strings valid for the call (ADR 0005 section 4).
unsafe fn read_prims(prims: *const BbUiPrim, count: u64) -> Result<Vec<UiPrim>, BbStatus> {
    if prims.is_null() || count == 0 || count > MAX_PRIMITIVES as u64 {
        return Err(BB_ERR_ARG);
    }
    (0..count as usize)
        // SAFETY: the caller guarantees `count` primitives at `prims`, and `count` is at least 1 (checked above).
        .map(|index| unsafe { read_prim(prims.add(index)) })
        .collect()
}

/// `register_theme`: a theme named by `name`, shown as `display`; its handle goes to `out_id`.
unsafe extern "C" fn register_theme(ctx: *mut c_void, name: BbStr, display: BbStr, out_id: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn UiBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's strings are valid for the call (ADR 0005 section 4).
        let name = unsafe { name_of(name) }?;
        // SAFETY: as above.
        let display = unsafe { text_within(display, MAX_NAME_BYTES) }?;
        let handle = slot.backend.register_theme(&slot.owner, &UiThemeSpec { name, display })?;
        slot.remember(handle);
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `set_slot`: the draw spec of one slot of a theme.
unsafe extern "C" fn set_slot(
    ctx: *mut c_void,
    theme: u64,
    slot_no: u32,
    prims: *const BbUiPrim,
    count: u64,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn UiBackend>(ctx)? };
        // SAFETY: the mod's list is valid for the call (see `read_prims`).
        let list = unsafe { read_prims(prims, count) }?;
        slot.backend.set_slot(&slot.owner, theme, slot_no, &list)
    })
}

/// `unregister_theme`: remove one theme.
unsafe extern "C" fn unregister_theme(ctx: *mut c_void, theme: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn UiBackend>(ctx)? };
        slot.backend.unregister_theme(&slot.owner, theme)?;
        slot.forget(theme);
        Ok(())
    })
}
