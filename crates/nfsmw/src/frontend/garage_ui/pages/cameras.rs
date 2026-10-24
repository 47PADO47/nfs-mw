//! Frontend collection names: package stem plus the original decimal category ID.

use super::{Page, Part, Row, Section};

pub(super) fn backroom(page: Page) -> &'static str {
    match page {
        Page::Categories { section: Section::Parts, .. } => "customize_customizecategory_backroom_2049",
        Page::Categories { section: Section::Performance, .. } => "customize_customizecategory_backroom_2050",
        Page::Categories { section: Section::Visual, .. } => "customize_customizecategory_backroom_2051",
        Page::Choices { row: Row::Part(Part::Body), .. } => "customize_customizeparts_backroom_257",
        Page::Choices { row: Row::Part(Part::Hood), .. } => "customize_customizeparts_backroom_260",
        Page::Choices { row: Row::Part(Part::Roof), .. } => "customize_customizeparts_backroom_261",
        Page::Choices { row: Row::Part(Part::Spoiler), .. } | Page::Spoilers { .. } => {
            "customize_spoilers_backroom_258"
        }
        Page::Choices { row: Row::Gauge, .. } => "customize_customhud_backroom_775",
        Page::Choices { row: Row::Performance(category) | Row::Junkman(category), .. } => {
            PERFORMANCE_BACKROOM[category as usize]
        }
        Page::VinylGroups { .. } => "customize_customizegenerictop_backroom_770",
        Page::Vinyls { group, .. } => VINYL_BACKROOM[usize::from(group).min(7)],
        Page::Paint { color: 0, .. } => "customize_paint_backroom_769",
        Page::Paint { color: 1, .. } => "customize_paint_backroom_771",
        Page::Paint { .. } => "customize_paint_backroom_770",
        Page::RimBrands { .. } => "customize_customizegenerictop_backroom_259",
        Page::Rims { .. } => "customize_rims_backroom",
        Page::GaugeColors { .. } => "customize_customhudcolor_backroom",
        Page::DecalZones { .. } => "customize_customizegenerictop_backroom_773",
        Page::DecalSlots { zone, .. } => decal_slots(zone, true),
        Page::Choices { row: Row::Decal(zone, _), .. } => decals(zone, true),
        Page::Cart { .. } => "customize_shoppingcart_backroom",
        _ => "customize_customizemain",
    }
}

pub(super) fn decals(zone: usize, backroom: bool) -> &'static str {
    let names = match backroom {
        false => [
            "customize_decals_1281",
            "customize_decals_1282",
            "customize_decals_1283",
            "customize_decals_1284",
            "customize_decals_1285",
            "customize_decals_1286",
        ],
        true => [
            "customize_decals_backroom_1281",
            "customize_decals_backroom_1282",
            "customize_decals_backroom_1283",
            "customize_decals_backroom_1284",
            "customize_decals_backroom_1285",
            "customize_decals_backroom_1286",
        ],
    };
    names.get(zone).copied().unwrap_or("customize_decals")
}

pub(super) fn decal_slots(zone: usize, backroom: bool) -> &'static str {
    let names = match backroom {
        false => [
            "customize_customizegenerictop_1281",
            "customize_customizegenerictop_1282",
            "customize_customizegenerictop_1283",
            "customize_customizegenerictop_1284",
            "customize_customizegenerictop_1285",
            "customize_customizegenerictop_1286",
        ],
        true => [
            "customize_customizegenerictop_backroom_1281",
            "customize_customizegenerictop_backroom_1282",
            "customize_customizegenerictop_backroom_1283",
            "customize_customizegenerictop_backroom_1284",
            "customize_customizegenerictop_backroom_1285",
            "customize_customizegenerictop_backroom_1286",
        ],
    };
    names.get(zone).copied().unwrap_or("customize_customizegenerictop")
}

#[cfg(test)]
mod tests;

pub(super) const PERFORMANCE: [&str; 7] = [
    "customize_customizeperformance_517",
    "customize_customizeperformance_518",
    "customize_customizeperformance_515",
    "customize_customizeperformance_514",
    "customize_customizeperformance_513",
    "customize_customizeperformance_519",
    "customize_customizeperformance_516",
];
const PERFORMANCE_BACKROOM: [&str; 7] = [
    "customize_customizeperformance_backroom_517",
    "customize_customizeperformance_backroom_518",
    "customize_customizeperformance_backroom_515",
    "customize_customizeperformance_backroom_514",
    "customize_customizeperformance_backroom_513",
    "customize_customizeperformance_backroom_519",
    "customize_customizeperformance_backroom_516",
];
const VINYL_BACKROOM: [&str; 8] = [
    "customize_customizeparts_backroom_1026",
    "customize_customizeparts_backroom_1027",
    "customize_customizeparts_backroom_1028",
    "customize_customizeparts_backroom_1029",
    "customize_customizeparts_backroom_1030",
    "customize_customizeparts_backroom_1031",
    "customize_customizeparts_backroom_1032",
    "customize_customizeparts_backroom_1033",
];
