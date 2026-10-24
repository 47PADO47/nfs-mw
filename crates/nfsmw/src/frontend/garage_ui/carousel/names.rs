//! Native choice labels; diagnostic row names stay outside the original screen bindings.

use super::super::{Page, choices::Item};
use crate::frontend::{ids, logic::Cx};
use crate::garage::{Model, menu_rows::Row};
use crate::ui::Text;
use nfsmw_data::car::customization::Category as Part;

pub(super) fn refresh(cx: &mut Cx, page: Page, item: &Item, model: &Model) {
    if matches!(item.row, Some(Row::Performance(_) | Row::Junkman(_))) {
        return;
    }
    let label = match item.row.zip(item.option) {
        Some((row, option)) => part(model, row, option, &item.label),
        None => category(page, item.next, model).unwrap_or_else(|| item.label.clone()),
    };
    for hash in [ids::ICON_TITLE, ids::ICON_TITLE_SHADOW] {
        cx.text_object(hash, &label);
    }
}

fn part(model: &Model, row: Row, option: usize, fallback: &str) -> String {
    let stock = || game(0x60a6_62f5).unwrap_or_else(|| "Stock".into());
    let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) else { return fallback.into() };
    if let Row::Gauge = row {
        let skin = option
            .checked_sub(1)
            .and_then(|at| catalog.gauge_skins.iter().filter(|skin| **skin > 0).nth(at))
            .copied()
            .unwrap_or(0);
        return catalog
            .gauge_names
            .get(&skin)
            .map(|choice| choice.language_hash.and_then(game).unwrap_or_else(|| choice.label.clone()))
            .unwrap_or_else(stock);
    }
    let (hash, choice) = match row {
        Row::Part(part) => {
            let choice = option.checked_sub(1).and_then(|at| catalog.choices[part as usize].get(at));
            (choice.map(|choice| choice.hash).or(catalog.stock[part as usize]), choice)
        }
        Row::Decal(zone, _) => {
            let body = model.draft.parts.parts[Part::Body as usize].or(catalog.stock[Part::Body as usize]);
            let choice = option.checked_sub(1).and_then(|at| {
                catalog.decals[zone]
                    .choices
                    .iter()
                    .filter(|choice| catalog.decals[zone].supports(body, choice.hash))
                    .nth(at)
            });
            (choice.map(|choice| choice.hash), choice)
        }
        _ => return fallback.into(),
    };
    if option == 0 && matches!(row, Row::Part(Part::Vinyl | Part::Wheels) | Row::Decal(_, _)) {
        return stock();
    }
    let native = hash.and_then(|hash| catalog.native_names.get(&hash));
    let language = choice.and_then(|choice| choice.language_hash).or_else(|| native.and_then(|name| name.1));
    if let Some(label) = language.and_then(game) {
        return label;
    }
    if let Some((name, _)) = native {
        if matches!(row, Row::Part(Part::Wheels)) {
            let end = name.char_indices().rev().nth(5).map(|(at, _)| at).unwrap_or(name.len());
            return name[..end].to_owned();
        }
        return name.clone();
    }
    if option == 0 {
        return stock();
    }
    fallback.into()
}

fn category(page: Page, next: Option<Page>, model: &Model) -> Option<String> {
    let next = next?;
    let label = match (page, next) {
        (Page::RimBrands { .. }, _) => return None,
        (Page::VinylGroups { .. }, Page::Vinyls { group, .. }) => {
            *[0xd922_8fc6, 0x1e8d_885f, 0x1c61_9fd8, 0x9c1b_8935, 0x7956_f7b0, 0x2d5b_ff0f, 0x209a_9158, 0xcd05_7d21]
                .get(group as usize)?
        }
        (Page::DecalZones { .. }, Page::DecalSlots { zone, .. } | Page::Choices { row: Row::Decal(zone, _), .. }) => {
            *[0x301d_edd3, 0x48e6_ca49, 0x3436_7c86, 0xddf8_0259, 0x8a76_97d6, 0xb1f9_b0c9].get(zone)?
        }
        (Page::DecalSlots { .. }, Page::Choices { row: Row::Decal(_, position), .. }) => 0x7d21_2cfa + position as u32,
        _ => next.title_label(model)?,
    };
    game(label)
}

fn game(hash: u32) -> Option<String> {
    Text::Game(hash).resolve()
}
