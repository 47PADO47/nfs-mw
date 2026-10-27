//! Bind the original inspection artwork; the shared orbit footer supplies its controls.

use crate::frontend::logic::Cx;
use crate::garage::{Car, Model};
use blackbox_feng::fe_hash_upper;

pub(super) fn refresh(cx: &mut Cx, model: &Model) {
    cx.hide(0xd86a_acf8, true);
    let Some(car) = model.selected() else { return };
    let (maker, name) = pictures(cx, car);
    for (hash, texture) in [(0x3e01_ad1d, maker), (0xb05d_d708, name)] {
        if let Some(object) = cx.object(hash) {
            cx.rt.set_hidden(object, false);
            cx.rt.set_texture(object, texture);
        }
    }
}

/// The maker's logo and the model-name picture of `car`: a mod car's own, drawn from its mod's brand and name (the package has
/// none for it, and its generic pictures read `temp350`), else the game's.
pub(super) fn pictures(cx: &Cx, car: &Car) -> (u32, u32) {
    crate::frontend::car_title::pictures(cx.assets, &car.folder)
        .unwrap_or_else(|| (manufacturer(cx, &car.manufacturer), model_logo(cx, &car.physics_name)))
}

/// The logo of the brand `name`: a mod's, or the game's own, or the generic one.
pub(super) fn manufacturer(cx: &Cx, name: &str) -> u32 {
    crate::frontend::car_title::logo_of(cx.assets, name).unwrap_or_else(|| fe_hash_upper("GENERIC_LOGO_128"))
}

pub(super) fn model_logo(cx: &Cx, name: &str) -> u32 {
    let hash = fe_hash_upper(&format!("SECONDARY_LOGO_{name}"));
    match cx.assets.texture(hash) {
        Some(_) => hash,
        None => fe_hash_upper("GENERIC_LOGO_256"),
    }
}
