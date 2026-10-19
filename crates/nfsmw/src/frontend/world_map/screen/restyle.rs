//! Recolours and de-clutters the live `WorldMapMain.fng` screen, in place, so the map keeps its own fullscreen picture,
//! filter switch, button prompts (`PC_key_*`, `R1`/`L1`) and mouse behaviour — only the look changes.
//!
//! - [`transparency`] runs for every map (Original and Remastered): it hides the decorative stripe ("white lines") and
//!   makes the opaque background images translucent, so the frozen world shows through behind the map.
//! - [`dark`] runs under the Remastered theme: it tints the background images dark and the text light (keeping the
//!   translucency), and hides the screen's own legend chrome (the list container, the eye icon and its labels), which
//!   the themed legend replaces.

use blackbox_feng::ObjectRef;
use blackbox_feng::package::{ObjectDef, Package};

use super::pools;
use crate::frontend::logic::Cx;

/// The background fill the Remastered look tints the panels (they are white textures, so the tint is the colour). The
/// alpha is left to [`transparency`].
const DARK: [u8; 3] = [16, 18, 24];
/// The light colour the static text takes under the Remastered look.
const TEXT: [u8; 3] = [226, 230, 238];
/// The most opaque a background image is left, so the frozen world still shows through behind the map (the normal bg
/// layer: less transparent than before, with no separate panel behind the map content).
const TRANSLUCENT: u8 = 200;
/// The one dark scrim every big background panel takes (sides and centre alike), so no white is left and the frozen
/// world shows through. There is no separate, more-opaque centre panel.
const SCRIM: [u8; 4] = [10, 12, 16, 160];
/// The background textures that carry the screen's panels and frames.
const PANEL: &str = "BasePoly.tga";
/// The decorative stripe texture.
const STRIPE: &str = "BaseStripe.tga";

/// For every map: hides the stripe lines, the "Move Cursor" hint and the loading ring; caps the background opacity; and
/// tints the big background panels a dark scrim, so no white is left and the frozen world shows through.
pub fn transparency(cx: &mut Cx) {
    // The Move Cursor hint + loading ring and the visibility ("eye") toggle are removed on every map.
    for guid in [
        pools::MOVE_CURSOR_TEXT,
        pools::MOVE_CURSOR_ICON,
        pools::LOADING_RING,
        pools::EYE_ICON,
        pools::TOGGLE_TEXT,
        pools::HIDE_SHOW_TEXT,
    ] {
        if let Some(o) = cx.guid(guid) {
            cx.rt.set_hidden(o, true);
        }
    }
    let Some(def) = cx.rt.package(cx.package) else {
        return;
    };
    let mut stripes = Vec::new();
    let mut panels = Vec::new();
    for (index, object) in def.objects.iter().enumerate() {
        if object.string.is_some() {
            continue;
        }
        match texture(def, object) {
            Some(name) if name.eq_ignore_ascii_case(STRIPE) => stripes.push(index),
            Some(name) if name.eq_ignore_ascii_case(PANEL) && object.data.alpha() > TRANSLUCENT as i32 => {
                panels.push(index)
            }
            _ => {}
        }
    }
    for index in stripes {
        cx.rt.set_hidden(ObjectRef { package: cx.package, index }, true);
    }
    for index in panels {
        cx.rt.set_alpha(ObjectRef { package: cx.package, index }, TRANSLUCENT);
    }
    for guid in pools::BG_SIDES.into_iter().chain([pools::BG_CENTRE]) {
        if let Some(o) = cx.guid(guid) {
            cx.rt.set_colour_rgba(o, SCRIM);
        }
    }
}

/// The Remastered recolour: panels dark (keeping [`transparency`]'s alpha), text light, and the screen's own legend
/// chrome hidden.
pub fn dark(cx: &mut Cx) {
    let Some(def) = cx.rt.package(cx.package) else {
        return;
    };
    let mut panels = Vec::new();
    let mut texts = Vec::new();
    for (index, object) in def.objects.iter().enumerate() {
        if object.string.is_some() {
            texts.push(index);
            continue;
        }
        if texture(def, object).is_some_and(|name| name.eq_ignore_ascii_case(PANEL)) {
            panels.push(index);
        }
    }
    for index in panels {
        cx.rt.set_colour(ObjectRef { package: cx.package, index }, DARK);
    }
    for index in texts {
        cx.rt.set_colour(ObjectRef { package: cx.package, index }, TEXT);
    }
    for guid in [pools::LEGEND_PANEL, pools::LEGEND_HEADER] {
        if let Some(o) = cx.guid(guid) {
            cx.rt.set_hidden(o, true);
        }
    }
}

/// The texture an object draws, by name.
fn texture<'a>(def: &'a Package, object: &ObjectDef) -> Option<&'a str> {
    object.resource.and_then(|r| def.resources.get(r)).map(|r| r.name.as_str())
}
