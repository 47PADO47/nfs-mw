//! Bind the original choice status, performance descriptions, brands and stat strips.

use blackbox_feng::fe_hash_upper;
use nfsmw_data::car::physics::performance::PackageDetails;

use crate::frontend::logic::Cx;
use crate::garage::{Model, Selection, menu_rows::Row};
use crate::ui::Text;

/// `option` is the underlying row index, before brand/group filtering.
pub(super) fn refresh(cx: &mut Cx, model: &Model, row: Row, option: usize, baseline: &Selection, cart: &Selection) {
    let mut comparison = model.clone();
    comparison.draft = baseline.clone();
    let installed = match row {
        Row::Number(_) => {
            [2, 3].into_iter().all(|zone| model.draft.parts.decals[zone][6..8] == baseline.parts.decals[zone][6..8])
        }
        _ => row.options(&comparison).1 == option,
    };
    comparison.draft = cart.clone();
    let in_cart = !installed
        && match row {
            Row::Number(_) => {
                [2, 3].into_iter().all(|zone| model.draft.parts.decals[zone][6..8] == cart.parts.decals[zone][6..8])
            }
            _ => row.options(&comparison).1 == option,
        };
    let locked = model.selection_locked
        && (!matches!(row, Row::Number(_)) || model.draft.parts.decals[3][6..8].iter().all(Option::is_some));
    status(cx, installed, in_cart, locked);

    let Some(car) = model.selected() else { return };
    let details = match row {
        Row::Performance(category) => car.performance_details.packages[category as usize].get(option),
        Row::Junkman(category) => Some(&car.performance_details.junkman[category as usize]),
        _ => None,
    };
    if let Some(details) = details {
        performance(cx, &reward_details(model, row, *details));
    }
    if let Some((labels, _)) = details.is_none().then(|| row.options(model)) {
        cx.text_object(0x5e7b_09c9, labels.get(option).map_or("", String::as_str));
    }
    comparison_bars(cx, model, baseline);
}

fn status(cx: &mut Cx, installed: bool, cart: bool, locked: bool) {
    let status = cx.object(0xd058_2feb);
    if let Some(object) = status {
        cx.rt.set_hidden(object, !installed && !cart);
        reveal_marker(cx, object, installed || cart);
        if installed {
            cx.rt.set_texture(object, 0x696a_e039);
        }
        if cart {
            cx.rt.set_texture(object, 0x1a77_7e25);
        }
    }
    if let Some(object) = cx.object(0xcffb_7033) {
        cx.rt.set_hidden(object, !locked);
        reveal_marker(cx, object, locked);
        if locked {
            cx.rt.set_texture(object, 0xf057_4bb2);
        }
    }
}

fn reveal_marker(cx: &mut Cx, object: blackbox_feng::ObjectRef, shown: bool) {
    let key = format!("garage:marker:{:?}:{}", object.package, object.index);
    let previous = cx.memory.insert(key, usize::from(shown));
    if !shown {
        return;
    }
    // These native markers initialize with a zero-alpha/zero-scale hide script.
    // Clearing set_hidden alone does not make them visible. Replay the authored reveal once.
    if previous != Some(1) || cx.rt.script_of(object) == Some(0x7ab7_0d67) {
        cx.rt.run_script(object, 0x0280_164f);
    }
}

/// Exact RGB comparison for vinyl/gauge palettes, which are not body-paint row indexes.
pub(super) fn color_status(
    cx: &mut Cx,
    row: Row,
    candidate: [u8; 3],
    candidate_hash: Option<u32>,
    model: &Model,
    baseline: &Selection,
    cart: &Selection,
) {
    let installed = color_matches(row, candidate, candidate_hash, baseline);
    let in_cart = !installed && color_matches(row, candidate, candidate_hash, cart);
    status(cx, installed, in_cart, model.selection_locked);
}

fn color_matches(row: Row, rgb: [u8; 3], hash: Option<u32>, selection: &Selection) -> bool {
    let selected_hash = match row {
        Row::Color(index @ 2..=4) => selection.parts.vinyl_color_parts[index - 2],
        Row::Color(index @ 5..=7) => selection.gauges.color_parts[index - 5],
        _ => None,
    };
    selected_hash == hash && selection_color(row, selection) == Some(rgb)
}

fn selection_color(row: Row, selection: &Selection) -> Option<[u8; 3]> {
    match row {
        Row::Color(0) => selection.color,
        Row::Color(1) => selection.parts.rim_color,
        Row::Color(index @ 2..=4) => selection.parts.vinyl_colors[index - 2],
        Row::Color(5) => selection.gauges.backing,
        Row::Color(6) => selection.gauges.needle,
        Row::Color(7) => selection.gauges.digits,
        _ => None,
    }
}

fn reward_details(model: &Model, row: Row, mut details: PackageDetails) -> PackageDetails {
    use nfsmw_data::car::physics::PerformanceCategory;
    if !model.castrol_unlocked || row != Row::Performance(PerformanceCategory::Engine) {
        return details;
    }
    if !model.selected().is_some_and(|car| car.physics_name.eq_ignore_ascii_case("FORDGT")) {
        return details;
    }
    if details.descriptions[2] != fe_hash_upper("PD_ENGINE_4_3") {
        return details;
    }
    details.brands[2] = 0x0b95_d4df;
    details.descriptions[2] = fe_hash_upper("PD_ENGINE_4_3_CASTROL");
    details
}

fn performance(cx: &mut Cx, details: &PackageDetails) {
    let name = Text::Game(details.name).resolve().unwrap_or_default();
    for hash in [0x5e7b_09c9, crate::frontend::ids::ICON_TITLE, crate::frontend::ids::ICON_TITLE_SHADOW] {
        cx.text_object(hash, &name);
    }
    for line in 0..3 {
        let text =
            (details.descriptions[line] != 0).then(|| Text::Game(details.descriptions[line]).resolve()).flatten();
        let hash = fe_hash_upper(&format!("DETAIL_TEXT_LINE{}", line + 1));
        cx.hide(hash, text.is_none());
        cx.text_object(hash, text.as_deref().unwrap_or_default());
        cx.hide(fe_hash_upper(&format!("PERFORMANCE_DETAILS_ICON{}", line + 1)), text.is_none());
        if let Some(object) = cx.named(&format!("BRAND_ICON_{}", line + 1)) {
            let texture = details.brands[line];
            let exists = texture != 0 && cx.assets.texture(texture).is_some();
            cx.rt.set_hidden(object, !exists);
            if exists {
                cx.rt.set_texture(object, texture);
            }
        }
    }
}

/// My Cars compares its selected record with itself.
pub(super) fn bars(cx: &mut Cx, model: &Model) {
    comparison_bars(cx, model, &model.draft);
}

pub(super) fn comparison_bars(cx: &mut Cx, model: &Model, baseline: &Selection) {
    let ratings = model.selected().and_then(|car| car.performance_ratings);
    let selected = ratings.map(|ratings| ratings.rating(model.draft.performance));
    let installed = ratings.map(|ratings| {
        ratings.rating(match baseline.car == model.draft.car {
            true => baseline.performance,
            false => model.draft.performance,
        })
    });
    apply_bars(cx.rt, cx.package, selected, installed);
}

fn apply_bars(
    rt: &mut blackbox_feng::Runtime,
    package: blackbox_feng::PackageId,
    selected: Option<[f32; 3]>,
    installed: Option<[f32; 3]>,
) {
    for (index, name) in ["TOPSPEED", "HANDLING", "ACCELERATION"].into_iter().enumerate() {
        let Some(base) = rt.find(package, fe_hash_upper(&format!("BASE_{name}"))) else { continue };
        let Some(position) = rt.position(base) else { continue };
        let Some(size) = rt.size(base) else { continue };
        let left = position.x - size.x / 2.0;
        let top = position.y - size.y / 2.0;
        if let Some(fill) = rt.find(package, fe_hash_upper(&format!("FILLBAR_{name}"))) {
            let fraction = selected.map(|values| values[index].clamp(0.0, 1.0)).unwrap_or(0.0);
            let width = 160.0 * fraction;
            let height = rt.size(fill).map_or(size.y, |size| size.y);
            rt.set_hidden(fill, selected.is_none() || width <= 0.0);
            rt.set_size_xy(fill, width, height);
            rt.set_position_xy(fill, left + width / 2.0, top + height / 2.0);
            rt.set_uv(fill, [0.0, 0.0, width / 32.0, 1.0]);
        }
        if let Some(handle) = rt.find(package, fe_hash_upper(&format!("HANDLE_{name}"))) {
            rt.set_hidden(handle, installed.is_none());
            if let Some(values) = installed {
                let handle_size = rt.size(handle).unwrap_or(glam::Vec3::splat(32.0));
                rt.set_position_xy(
                    handle,
                    left + 160.0 * values[index].clamp(0.0, 1.0) + handle_size.x / 2.0,
                    top + handle_size.y / 2.0,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsmw_data::car::physics::PerformanceCategory;

    #[test]
    fn palette_status_keeps_distinct_custom_rgb_values() {
        let mut installed = Selection::default();
        installed.parts.vinyl_colors[0] = Some([12, 34, 56]);
        let mut cart = installed.clone();
        cart.parts.vinyl_colors[0] = Some([65, 43, 21]);
        assert_ne!(selection_color(Row::Color(2), &installed), selection_color(Row::Color(2), &cart));
        assert_eq!(selection_color(Row::Color(2), &installed), Some([12, 34, 56]));
        assert_eq!(selection_color(Row::Color(2), &cart), Some([65, 43, 21]));
        installed.parts.vinyl_color_parts[0] = Some(42);
        assert!(color_matches(Row::Color(2), [12, 34, 56], Some(42), &installed));
        assert!(!color_matches(Row::Color(2), [12, 34, 56], Some(43), &installed));
        assert!(!color_matches(Row::Color(2), [12, 34, 56], None, &installed));
    }

    #[test]
    fn castrol_display_requires_the_entitlement_and_ford_gt_engine_package() {
        let package = PackageDetails { descriptions: [0, 0, fe_hash_upper("PD_ENGINE_4_3")], ..Default::default() };
        let mut model = Model {
            cars: vec![crate::garage::Car {
                folder: "FORDGT".into(),
                physics_name: "fordgt".into(),
                ..Default::default()
            }],
            draft: Selection { car: "FORDGT".into(), ..Default::default() },
            ..Default::default()
        };
        let row = Row::Performance(PerformanceCategory::Engine);
        assert_eq!(reward_details(&model, row, package).descriptions, package.descriptions);
        model.castrol_unlocked = true;
        assert_eq!(reward_details(&model, row, package).brands[2], 0x0b95_d4df);
        assert_eq!(reward_details(&model, row, package).descriptions[2], fe_hash_upper("PD_ENGINE_4_3_CASTROL"));
        assert_eq!(reward_details(&model, Row::Performance(PerformanceCategory::Brakes), package).brands[2], 0);
        model.cars[0].physics_name = "rx7".into();
        assert_eq!(reward_details(&model, row, package).brands[2], 0);
    }
}

#[cfg(test)]
#[path = "details_bar_tests.rs"]
mod bar_tests;
