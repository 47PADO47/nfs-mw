//! Bind the installed header, costs, unlock notice and heat meter.

use super::Page;
use crate::frontend::{ids, logic::Cx};
use crate::garage::Model;
use blackbox_feng::fe_hash_upper;

/// A retained parent has finished its leave fade; replay its own entrance scripts on return.
pub(super) fn restore(cx: &mut Cx) {
    let init = fe_hash_upper("INIT");
    let objects: Vec<_> = cx
        .rt
        .package(cx.package)
        .into_iter()
        .flat_map(|p| p.objects.iter().enumerate())
        .filter(|(_, object)| object.scripts.iter().any(|script| script.id == init))
        .map(|(index, _)| blackbox_feng::ObjectRef { package: cx.package, index })
        .collect();
    for object in objects {
        cx.rt.run_script(object, init);
    }
}

pub(super) fn refresh(cx: &mut Cx, page: Page, model: &Model) {
    cx.hide(0xd058_2feb, true);
    cx.hide(0xcffb_7033, true);
    let title = page.title();
    let title_key = page.title_label(model);
    match title_key {
        Some(key) => {
            cx.label_group(ids::TITLE_GROUP, key);
            cx.label_group(0x2426_57ce, key);
        }
        None => {
            cx.text_group(ids::TITLE_GROUP, &title);
            cx.text_group(0x2426_57ce, &title);
        }
    }
    let error = model
        .shop_error
        .as_deref()
        .or_else(|| model.message.as_ref().and_then(|m| m.as_ref().err().map(String::as_str)));
    cx.hide(0xebc3_e6b7, error.is_none());
    cx.text_object(0xa629_8e25, error.unwrap_or_default());
    // Runtime text placeholders and the original's hidden global-sound/event objects are never user content.
    for hash in [0x8c4c_1d77, 0x8c4c_1d35, 0x8c4c_1cf3, 0x5b9d_88b9] {
        cx.hide(hash, true);
    }
    if let Some((current, preview)) = model.shop_heat {
        let meter = nfsmw_career::economy::cooling::HeatMeter::new(current, preview);
        for (index, segment) in meter.segments.into_iter().enumerate() {
            let name = fe_hash_upper(&format!("HEAT_BASE_LED_{}", index + 1));
            if let Some(object) = cx.object(name) {
                let alpha = match segment {
                    nfsmw_career::economy::cooling::Segment::Off => 30,
                    _ => 255,
                };
                cx.rt.set_alpha(object, alpha);
            }
        }
        for hash in [0x5ffe_e1d8, 0xe637_955c] {
            if let Some(object) = cx.object(hash) {
                cx.rt.set_texture(object, fe_hash_upper(&format!("HEAT_X{}", meter.multiplier)));
            }
        }
    }
    cx.hide(0xf249_2598, model.shop_heat.is_none());
    cx.hide(0x8d15_59a4, !model.shop);
    cx.hide(0x1e61_6c7a, !model.shop);
    cx.hide(0x5aec_8d91, true);
    cx.hide(0xdc6e_e739, !super::choices::can_enter_backroom(model));
}

pub(super) fn selection(cx: &mut Cx, label: &str, number: usize, count: usize) {
    for hash in [ids::ICON_TITLE, ids::ICON_TITLE_SHADOW] {
        cx.text_object(hash, label);
    }
    for hash in [0x5b80_5ee5, 0x40d2_3023] {
        cx.text_object(hash, &number.to_string());
    }
    for hash in [0x2e48_1779, 0x5e7b_2637] {
        cx.text_object(hash, &count.to_string());
    }
}

pub(super) fn text(cx: &mut Cx, name: &str, value: &str) {
    if let Some(object) = cx.named(name) {
        cx.rt.set_text(object, value);
    }
}

pub(super) fn focus(cx: &mut Cx, object: blackbox_feng::ObjectRef, selected: bool) {
    if native_focus(cx, object, selected) {
        return;
    }
    cx.rt.set_colour_rgba(
        object,
        match selected {
            true => [255, 174, 64, 255],
            false => [255; 4],
        },
    );
}

/// Some garage controls expose highlight scripts without generic gain/lose-focus responses.
pub(super) fn native_focus(cx: &mut Cx, object: blackbox_feng::ObjectRef, selected: bool) -> bool {
    let script = match selected {
        true => ids::SCRIPT_HIGHLIGHT,
        false => ids::SCRIPT_UNHIGHLIGHT,
    };
    let authored = cx
        .rt
        .package(object.package)
        .and_then(|package| package.objects.get(object.index))
        .is_some_and(|definition| definition.scripts.iter().any(|track| track.id == script));
    if authored {
        if cx.rt.script_of(object) != Some(script) {
            cx.rt.run_script(object, script);
        }
        return true;
    }
    false
}
