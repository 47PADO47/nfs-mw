//! Black/White tabs preserve native manufacturer identity while keeping Row option indices stable.

use super::{Item, Model, Row};
use nfsmw_data::car::customization::{Category, DecalVariant};

pub(in crate::frontend::garage_ui) fn initial_black(model: &Model, row: Row) -> bool {
    let Row::Decal(zone, position) = row else { return true };
    let hash = model.draft.parts.decals[zone][position];
    hash.and_then(|hash| model.selected()?.parts.as_ref()?.decals[zone].variants.get(&hash))
        .is_none_or(|variant| variant.matches(true))
}

/// The option belongs to the underlying unfiltered Row, with removal at zero.
pub(in crate::frontend::garage_ui) fn brand(model: &Model, row: Row, option: usize) -> Option<u32> {
    variant(model, row, option).map(|variant| variant.brand)
}

pub(in crate::frontend::garage_ui) fn filter(
    model: &Model,
    row: Row,
    black: bool,
    entries: &mut Vec<Item>,
    preserve_brand: Option<u32>,
) -> usize {
    entries.retain(|item| {
        let Some(option) = item.option else { return false };
        option == 0 || variant(model, row, option).is_some_and(|variant| variant.matches(black))
    });
    preserve_brand
        .and_then(|selected| {
            entries.iter().rposition(|item| item.option.and_then(|option| brand(model, row, option)) == Some(selected))
        })
        .unwrap_or(0)
}

fn variant(model: &Model, row: Row, option: usize) -> Option<DecalVariant> {
    let Row::Decal(zone, _) = row else { return None };
    let index = option.checked_sub(1)?;
    let catalog = model.selected()?.parts.as_ref()?;
    let body = model.draft.parts.parts[Category::Body as usize].or(catalog.stock[Category::Body as usize]);
    let decals = &catalog.decals[zone];
    let choice = decals.choices.iter().filter(|choice| decals.supports(body, choice.hash)).nth(index)?;
    decals.variants.get(&choice.hash).copied()
}

#[cfg(test)]
mod tests;
