//! Bind the installed CustomHUD/CustomHUDColor gauge sample to the selected skin and colours.

use blackbox_feng::{PackageId, Runtime, fe_hash_upper};

use crate::frontend::logic::Cx;
use crate::garage::{GaugeSelection, Model};
use crate::hud::skin;

const FACE: u32 = 0xc072_1eb9;
const FILL: u32 = 0x05d1_9f25;
const NEEDLE: u32 = 0xd312_f0cb;
const TURBO_FACE: u32 = 0xc62a_d685;
const TURBO_NEEDLE: u32 = 0x8fe2_a217;
const TURBO_GROUP: u32 = 0xc5d5_51b7;
const UNITS: u32 = 0xb8f1_f802;
const DIGITS: &[u32] = &[UNITS, 0xb845_89be, 0x94de_0412, 0x94de_0411, 0x94de_0410, 0x4113_f134, 0x0709_1622];

/// The installed sample starts hidden; its authored entrance runs once per page activation.
pub(super) fn show(cx: &mut Cx) {
    show_sample(cx.rt, cx.package);
}

fn show_sample(rt: &mut Runtime, package: PackageId) {
    if let Some(group) = rt.find(package, 0xdee8_632b) {
        rt.set_hidden(group, false);
        rt.run_script(group, 0x5079_c8f8);
    }
}

pub(super) fn refresh(cx: &mut Cx, model: &Model) {
    let assets = cx.assets;
    apply(cx.rt, cx.package, &model.draft.gauges, model.gauge_max_rpm, model.gauge_has_turbo, |hash| {
        assets.texture(hash).is_some()
    });
    if let Some(units) = cx.object(UNITS) {
        cx.rt.set_label(
            units,
            match cx.settings.speed_units {
                crate::settings::SpeedUnits::Imperial => 0x8569_ab44,
                crate::settings::SpeedUnits::Metric => 0x8569_a25f,
            },
        );
    }
}

fn apply(
    rt: &mut Runtime,
    package: PackageId,
    gauges: &GaugeSelection,
    max_rpm: f32,
    has_turbo: bool,
    available: impl Fn(u32) -> bool,
) {
    let skin = gauges.skin;
    for (name, chosen, fallback) in [
        (FACE, skin::tach_face_texture(max_rpm, skin), skin::tach_face_texture(max_rpm, 0)),
        (FILL, skin::fill_texture(skin), skin::fill_texture(0)),
        (NEEDLE, skin::needle_texture(skin), skin::needle_texture(0)),
        (TURBO_FACE, fe_hash_upper(&format!("TURBO_LINES_{skin:02}")), fe_hash_upper("TURBO_LINES_00")),
        (TURBO_NEEDLE, fe_hash_upper(&format!("TURBO_NEEDLE_{skin:02}")), fe_hash_upper("TURBO_NEEDLE_00")),
    ] {
        let Some(object) = rt.find(package, name) else { continue };
        if available(chosen) {
            rt.set_texture(object, chosen);
            continue;
        }
        if available(fallback) {
            rt.set_texture(object, fallback);
        }
    }
    if let Some(turbo) = rt.find(package, TURBO_GROUP) {
        rt.set_hidden(turbo, !has_turbo);
    }
    // Set RGB after package animation; authored alpha still controls the sample's fades and dimmed digit backing.
    for (objects, color) in [
        (&[FACE, TURBO_FACE][..], gauges.backing),
        (&[NEEDLE, TURBO_NEEDLE][..], gauges.needle),
        (DIGITS, gauges.digits),
    ] {
        for &name in objects {
            let Some(object) = rt.find(package, name) else { continue };
            let authored = rt.package(package).and_then(|package| package.objects.get(object.index)).map(|object| {
                let rgba = object.data.colour_rgba();
                [rgba[0], rgba[1], rgba[2]]
            });
            if let Some(rgb) = color.or(authored) {
                rt.set_colour(object, rgb);
            }
        }
    }
}

#[cfg(test)]
#[path = "gauge_preview_tests.rs"]
mod tests;
