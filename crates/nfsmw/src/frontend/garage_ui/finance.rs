//! Original cash and marker fields, separating the preview from the committed cart.

use super::{Page, Section};
use crate::frontend::logic::Cx;
use crate::garage::{Model, Selection, menu_rows::Row, pricing::Totals};
use blackbox_feng::{PackageId, Runtime};
use nfsmw_data::car::customization::Category as Part;

#[derive(Default)]
struct Frame {
    cash: Option<u32>,
    selected: Option<(u32, u32)>,
    totals: Option<Totals>,
    backroom: bool,
    markers: u32,
    available: u32,
    marker_label: u32,
    category: bool,
    marker_rows: Vec<(i64, u32)>,
    marker_summary: Option<(u32, u32)>,
}

pub(super) fn refresh(
    cx: &mut Cx,
    page: Page,
    model: &Model,
    baseline: &Selection,
    cart: &Selection,
    section: Option<Section>,
) {
    apply(cx.rt, cx.package, &frame(page, model, baseline, cart, section));
}

fn frame(page: Page, model: &Model, baseline: &Selection, cart: &Selection, section: Option<Section>) -> Frame {
    let mut frame = Frame {
        cash: model.shop_cash,
        backroom: model.backroom,
        markers: model.shop_markers.unwrap_or_default(),
        ..Default::default()
    };
    if !model.shop {
        return frame;
    }
    let Some(pricing) = model.pricing.as_ref() else { return frame };
    let Some(car) = model.selected() else { return frame };
    let active = match page {
        Page::Cart { .. } => &model.draft,
        _ => cart,
    };
    let items = pricing.items(car, baseline, active, model.backroom, true).ok();
    frame.totals = items.as_deref().map(Totals::of);
    if model.backroom && matches!(page, Page::Cart { .. }) {
        let (kinds, icons, labels): (&[usize], &[u32], _) = match section.unwrap_or(Section::Visual) {
            Section::Parts => (
                &[8, 9, 10, 11, 12],
                &[0xaf39_3dba, 0xf375_276e, 0xc51a_4f62, 0xc194_91cc, 0x25a4_375e],
                (0xa03a_752f, 0x4ac6_8298),
            ),
            Section::Performance => (
                &[1, 2, 3, 4, 5, 6, 7],
                &[0x4887_f351, 0x4f42_4e0f, 0x6fea_04c8, 0x8e28_4227, 0x0019_0eb6, 0x7373_f1ef, 0xd142_d3e3],
                (0x358d_b897, 0x6834_2700),
            ),
            Section::Visual => {
                (&[14, 15, 16, 13], &[0xd35f_04c0, 0xa913_5927, 0x0db8_9e17, 0x8ba6_02fc], (0x9329_6e59, 0x78f1_c602))
            }
        };
        frame.markers = kinds.iter().map(|kind| model.shop_marker_counts[*kind]).sum();
        frame.marker_rows = kinds
            .iter()
            .zip(icons)
            .map(|(kind, icon)| {
                let spent = items
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .filter(|item| item.marker.is_some_and(|marker| marker.0 as usize == *kind))
                    .count();
                (i64::from(model.shop_marker_counts[*kind]) - spent as i64, *icon)
            })
            .collect();
        frame.marker_summary = Some(labels);
    }
    if let Some(row) = row(page) {
        frame.selected =
            pricing.item(row, &model.draft, baseline, car, model.backroom).ok().map(|item| (item.cost, item.trade_in));
    }
    let (kinds, label) = marker_category(page, car.performance_details.turbo);
    frame.available = kinds.iter().map(|kind| model.shop_marker_counts[*kind]).sum();
    frame.marker_label = label;
    frame.category = matches!(
        page,
        Page::Main
            | Page::Categories { .. }
            | Page::RimBrands { .. }
            | Page::VinylGroups { .. }
            | Page::DecalZones { .. }
            | Page::DecalSlots { .. }
    );
    frame
}

fn row(page: Page) -> Option<Row> {
    Some(match page {
        Page::Choices { row, .. } => row,
        Page::Rims { .. } => Row::Part(Part::Wheels),
        Page::Spoilers { .. } => Row::Part(Part::Spoiler),
        Page::Vinyls { .. } | Page::Paint { color: 2..=4, .. } => Row::Part(Part::Vinyl),
        Page::Paint { color: 0, .. } => Row::Part(Part::Paint),
        Page::Paint { color: 1, .. } => Row::Part(Part::RimPaint),
        Page::GaugeColors { .. } => Row::Gauge,
        Page::Numbers => Row::Number(0),
        _ => return None,
    })
}

fn marker_category(page: Page, turbo: bool) -> (&'static [usize], u32) {
    use nfsmw_data::car::physics::PerformanceCategory as Hardware;
    match page {
        Page::Categories { section: Section::Parts, .. } => return (&[8, 9, 10, 11, 12, 13], 0xd3a2_fbe1),
        Page::Categories { section: Section::Performance, .. } => return (&[1, 2, 3, 4, 5, 6, 7], 0x3c27_a989),
        Page::Categories { section: Section::Visual, .. } => return (&[14, 15, 16], 0x5692_be6b),
        Page::RimBrands { .. } => return (&[11], 0x8a4b_fbf2),
        Page::VinylGroups { .. } => return (&[14], 0xd413_e189),
        Page::DecalZones { .. } | Page::DecalSlots { .. } => return (&[15], 0xd2cb_c510),
        _ => {}
    }
    match row(page) {
        Some(Row::Part(Part::Body)) => (&[8], 0x7c50_498c),
        Some(Row::Part(Part::Hood)) => (&[9], 0x8a46_99e1),
        Some(Row::Part(Part::Spoiler)) => (&[10], 0x5201_2995),
        Some(Row::Part(Part::Wheels)) => (&[11], 0x8a4b_fbf2),
        Some(Row::Part(Part::Roof)) => (&[12], 0x8301_00f0),
        Some(Row::Part(Part::Vinyl)) => (&[14], 0xd413_e189),
        Some(Row::Part(Part::Paint)) => (&[16], 0xd3a2_d4d3),
        Some(Row::Gauge) => (&[13], 0xc253_ec92),
        Some(Row::Decal(_, _)) => (&[15], 0xd2cb_c510),
        Some(Row::Performance(category) | Row::Junkman(category)) => match category {
            Hardware::Brakes => (&[1], 0x2884_658f),
            Hardware::Engine => (&[2], 0x2f3e_c04d),
            Hardware::Nitrous => (&[3], 0xc129_562b),
            Hardware::Induction if turbo => (&[4], 0xd3f6_5323),
            Hardware::Induction => (&[4], 0x63a5_1aa2),
            Hardware::Chassis => (&[5], 0xb7cb_fcce),
            Hardware::Tires => (&[6], 0xd3ef_befe),
            Hardware::Transmission => (&[7], 0xd1e7_7ca1),
        },
        _ => (&[], 0),
    }
}

fn apply(rt: &mut Runtime, package: PackageId, frame: &Frame) {
    let money = |value: i64| {
        let amount = crate::i18n::fmt::grouped(value.unsigned_abs().min(u64::from(u32::MAX)) as i64);
        match value < 0 {
            true => format!("-{amount}"),
            false => amount,
        }
    };
    let mut text = |hashes: &[u32], value: String| {
        for hash in hashes {
            if let Some(object) = rt.find(package, *hash) {
                set_text(rt, object, &value);
            }
        }
    };
    text(&[0xa2a7_79b6, 0x2922_f251], frame.cash.map(|value| money(i64::from(value))).unwrap_or_default());
    text(&[0x6dfa_e796, 0x20c8_3c31], frame.selected.map(|value| money(i64::from(value.0))).unwrap_or_default());
    text(&[0xe83b_28e3, 0x2f5f_0a1e], frame.selected.map(|value| money(i64::from(value.1))).unwrap_or_default());
    text(&[0x3614_0053, 0xc3a5_7dc5], frame.totals.map(|value| money(value.net)).unwrap_or_default());
    if let Some(totals) = frame.totals {
        let (cost, total) = match frame.backroom {
            true => (totals.markers as i64, i64::from(frame.markers) - totals.markers as i64),
            false => (totals.cost as i64, totals.net),
        };
        text(&[0xd149_7a06], money(cost));
        text(&[0x34f7_c0e8], money(-(totals.trade_in as i64)));
        text(&[0x1866_1565], money(total));
        text(&[0x8531_e22e], frame.cash.map(|cash| money(i64::from(cash) - totals.net)).unwrap_or_default());
    }
    text(
        &[0x23d9_18fe],
        match frame.category {
            true => frame.markers,
            false => 1,
        }
        .to_string(),
    );
    text(&[0x83e3_cd39], frame.available.to_string());
    if let Some(object) = rt.find(package, 0x63ca_8308) {
        set_label(rt, object, frame.marker_label);
    }
    if let Some(object) = rt.find(package, 0x9ea2_2e0b) {
        rt.set_hidden(object, frame.cash.is_none());
    }
    if let Some((first, second)) = frame.marker_summary {
        for (hash, label) in [(0x8cdc_b8ed, first), (0xd3d3_b1f4, second)] {
            if let Some(object) = rt.find(package, hash) {
                set_label(rt, object, label);
            }
        }
        for index in 0..7 {
            let row = frame.marker_rows.get(index);
            let number = index + 1;
            if let Some(object) = rt.find(package, blackbox_feng::fe_hash_upper(&format!("MARKER_GROUP_{number}"))) {
                rt.set_hidden(object, row.is_none());
                let script = match row.is_some_and(|row| row.0 == 0) {
                    true => "GREY",
                    false => "NORMAL",
                };
                let desired = blackbox_feng::fe_hash_upper(script);
                if rt.script_of(object) != Some(desired) {
                    rt.run_script(object, desired);
                }
            }
            if let Some((remaining, icon)) = row {
                for name in [format!("MARKER_NUM_{number}"), format!("MARKER_BLOOM_{number}")] {
                    if let Some(object) = rt.find(package, blackbox_feng::fe_hash_upper(&name)) {
                        set_text(rt, object, &money(*remaining));
                    }
                }
                if let Some(object) = rt.find(package, 0xeb95_7471 + index as u32) {
                    rt.set_texture(object, *icon);
                }
            }
        }
    }
}

fn strings(rt: &Runtime, object: blackbox_feng::ObjectRef) -> Vec<blackbox_feng::ObjectRef> {
    let Some(package) = rt.package(object.package) else { return Vec::new() };
    let def = &package.objects[object.index];
    if def.string.is_some() {
        return vec![object];
    }
    package
        .objects
        .iter()
        .enumerate()
        .filter(|(_, child)| child.parent == Some(def.guid) && child.string.is_some())
        .map(|(index, _)| blackbox_feng::ObjectRef { package: object.package, index })
        .collect()
}

fn set_text(rt: &mut Runtime, object: blackbox_feng::ObjectRef, value: &str) {
    for string in strings(rt, object) {
        rt.set_text(string, value);
    }
}

fn set_label(rt: &mut Runtime, object: blackbox_feng::ObjectRef, label: u32) {
    for string in strings(rt, object) {
        rt.set_label(string, label);
    }
}

#[cfg(test)]
#[path = "finance_tests.rs"]
mod tests;
