//! Bind the original inspection artwork; the shared orbit footer supplies its controls.

use crate::frontend::logic::Cx;
use crate::garage::Model;
use blackbox_feng::fe_hash_upper;

pub(super) fn refresh(cx: &mut Cx, model: &Model) {
    cx.hide(0xd86a_acf8, true);
    let Some(car) = model.selected() else { return };
    for (name, texture) in
        [(0x3e01_ad1d, manufacturer(cx, &car.manufacturer)), (0xb05d_d708, model_logo(cx, &car.physics_name))]
    {
        if let Some(object) = cx.object(name) {
            cx.rt.set_hidden(object, false);
            cx.rt.set_texture(object, texture);
        }
    }
}

pub(super) fn manufacturer(cx: &Cx, name: &str) -> u32 {
    let cleaned: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let hash = fe_hash_upper(&format!("CARSELECT_MANUFACTURER_{cleaned}"));
    match cx.assets.texture(hash) {
        Some(_) => hash,
        None => fe_hash_upper("GENERIC_LOGO_128"),
    }
}

pub(super) fn model_logo(cx: &Cx, name: &str) -> u32 {
    let hash = fe_hash_upper(&format!("SECONDARY_LOGO_{name}"));
    match cx.assets.texture(hash) {
        Some(_) => hash,
        None => fe_hash_upper("GENERIC_LOGO_256"),
    }
}
