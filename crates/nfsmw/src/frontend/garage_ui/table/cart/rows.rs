//! Identify cart changes and remove exactly one installed item.

use crate::garage::{Model, Selection, menu_rows::Row};
use nfsmw_data::car::{customization::Category as Part, physics::PerformanceCategory as Hardware};

pub(super) fn changed(_model: &Model, baseline: &Selection, cart: &Selection) -> Vec<Row> {
    let mut rows = Vec::new();
    for part in Part::ALL {
        if baseline.parts.parts[part as usize] != cart.parts.parts[part as usize] {
            rows.push(Row::Part(part));
        }
    }
    for category in Hardware::ALL {
        if baseline.performance.get(category) != cart.performance.get(category) {
            rows.push(Row::Performance(category));
        }
        if baseline.performance.has_junkman(category) != cart.performance.has_junkman(category) {
            rows.push(Row::Junkman(category));
        }
    }
    if baseline.gauges.skin != cart.gauges.skin {
        rows.push(Row::Gauge);
    }
    for (part, changed) in [
        (Part::Paint, baseline.color != cart.color),
        (Part::RimPaint, baseline.parts.rim_color != cart.parts.rim_color),
        (
            Part::Vinyl,
            baseline.parts.vinyl_colors != cart.parts.vinyl_colors
                || baseline.parts.vinyl_color_parts != cart.parts.vinyl_color_parts,
        ),
    ] {
        if changed && !rows.contains(&Row::Part(part)) {
            rows.push(Row::Part(part));
        }
    }
    if baseline.gauges != cart.gauges && !rows.contains(&Row::Gauge) {
        rows.push(Row::Gauge);
    }
    for zone in 0..6 {
        let limit = match zone {
            2 | 3 => 6,
            _ => 8,
        };
        for position in 0..limit {
            if baseline.parts.decals[zone][position] != cart.parts.decals[zone][position] {
                rows.push(Row::Decal(zone, position));
            }
        }
    }
    if [2, 3].into_iter().any(|zone| baseline.parts.decals[zone][6..8] != cart.parts.decals[zone][6..8]) {
        rows.push(Row::Number(0));
    }
    rows
}

pub(super) fn active(model: &Model, cart: &Selection, row: Row) -> bool {
    !changed(model, &model.draft, cart).contains(&row)
}

pub(super) fn restore(row: Row, baseline: &Selection, cart: &mut Selection) {
    match row {
        Row::Part(part) => {
            cart.parts.parts[part as usize] = baseline.parts.parts[part as usize];
            match part {
                Part::Paint => cart.color = baseline.color,
                Part::RimPaint => cart.parts.rim_color = baseline.parts.rim_color,
                Part::Vinyl => {
                    cart.parts.vinyl_colors = baseline.parts.vinyl_colors;
                    cart.parts.vinyl_color_parts = baseline.parts.vinyl_color_parts;
                }
                Part::Wheels if baseline.parts.parts[Part::Wheels as usize].is_none() => {
                    cart.parts.parts[Part::RimPaint as usize] = baseline.parts.parts[Part::RimPaint as usize];
                    cart.parts.rim_color = baseline.parts.rim_color;
                }
                _ => {}
            }
        }
        Row::Performance(part) => cart.performance.levels[part as usize] = baseline.performance.get(part),
        Row::Junkman(part) => {
            let mask = 1 << part as u8;
            cart.performance.junkman = (cart.performance.junkman & !mask) | (baseline.performance.junkman & mask);
        }
        Row::Gauge => cart.gauges = baseline.gauges.clone(),
        Row::Color(0) => cart.color = baseline.color,
        Row::Color(1) => cart.parts.rim_color = baseline.parts.rim_color,
        Row::Color(index @ 2..=4) => {
            cart.parts.vinyl_colors[index - 2] = baseline.parts.vinyl_colors[index - 2];
            cart.parts.vinyl_color_parts[index - 2] = baseline.parts.vinyl_color_parts[index - 2];
        }
        Row::Color(index @ 5..=7) => {
            match index {
                5 => cart.gauges.backing = baseline.gauges.backing,
                6 => cart.gauges.needle = baseline.gauges.needle,
                _ => cart.gauges.digits = baseline.gauges.digits,
            }
            cart.gauges.color_parts[index - 5] = baseline.gauges.color_parts[index - 5];
        }
        Row::Decal(zone, position) => cart.parts.decals[zone][position] = baseline.parts.decals[zone][position],
        Row::Number(_) => {
            for zone in [2, 3] {
                cart.parts.decals[zone][6..8].copy_from_slice(&baseline.parts.decals[zone][6..8]);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_number_digit_is_one_cart_entry_and_restores_both_doors() {
        let model = Model::default();
        let mut baseline = Selection::default();
        for zone in [2, 3] {
            baseline.parts.decals[zone][6..8].copy_from_slice(&[Some(10), Some(20)]);
        }
        let mut cart = baseline.clone();
        for zone in [2, 3] {
            cart.parts.decals[zone][7] = Some(21);
        }
        assert_eq!(changed(&model, &baseline, &cart), [Row::Number(0)]);
        restore(Row::Number(0), &baseline, &mut cart);
        assert_eq!(cart, baseline);
    }

    #[test]
    fn native_color_identity_changes_are_part_of_the_parent_cart_entry() {
        let model = Model::default();
        let mut baseline = Selection::default();
        baseline.parts.vinyl_colors[0] = Some([10, 20, 30]);
        baseline.parts.vinyl_color_parts[0] = Some(10);
        let mut cart = baseline.clone();
        cart.parts.vinyl_color_parts[0] = Some(11);
        cart.gauges.color_parts[1] = Some(12);
        cart.gauges.needle = Some([10, 20, 30]);
        assert_eq!(changed(&model, &baseline, &cart), [Row::Part(Part::Vinyl), Row::Gauge]);
        restore(Row::Part(Part::Vinyl), &baseline, &mut cart);
        restore(Row::Gauge, &baseline, &mut cart);
        assert_eq!(cart, baseline);
    }

    #[test]
    fn unchecking_a_body_kit_keeps_independent_decal_choices() {
        let mut baseline = Selection::default();
        baseline.parts.parts[Part::Body as usize] = Some(1);
        let mut cart = baseline.clone();
        cart.parts.parts[Part::Body as usize] = Some(2);
        cart.parts.decals[0][0] = Some(99);
        restore(Row::Part(Part::Body), &baseline, &mut cart);
        assert_eq!(cart.parts.decals[0][0], Some(99));
        assert_eq!(cart.parts.parts[Part::Body as usize], Some(1));
    }
}
