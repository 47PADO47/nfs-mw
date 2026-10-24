//! Backroom categories are omitted when the career has no matching native marker.

use super::{Hardware, Item, Model, Page, Part, Row};

pub(super) fn entry(parent: Page, item: &Item, model: &Model) -> bool {
    if let Some(Page::Categories { .. }) = item.next {
        return super::items(item.next.unwrap(), model).iter().any(|item| item.enabled);
    }
    let page = item.next.unwrap_or(parent);
    let row = match page {
        Page::Choices { row, .. } => row,
        Page::RimBrands { .. } | Page::Rims { .. } => Row::Part(Part::Wheels),
        Page::Spoilers { .. } => Row::Part(Part::Spoiler),
        Page::VinylGroups { .. } | Page::Vinyls { .. } => Row::Part(Part::Vinyl),
        Page::Paint { color: 0, .. } => Row::Part(Part::Paint),
        Page::Paint { color: 2..=4, .. } => Row::Part(Part::Vinyl),
        Page::GaugeColors { .. } => Row::Gauge,
        Page::DecalZones { .. } | Page::DecalSlots { .. } => Row::Decal(0, 0),
        _ => return true,
    };
    let kind = match row {
        Row::Performance(category) | Row::Junkman(category) => match category {
            Hardware::Brakes => 1,
            Hardware::Engine => 2,
            Hardware::Nitrous => 3,
            Hardware::Induction => 4,
            Hardware::Chassis => 5,
            Hardware::Tires => 6,
            Hardware::Transmission => 7,
        },
        Row::Part(part) => match part {
            Part::Body => 8,
            Part::Hood => 9,
            Part::Spoiler => 10,
            Part::Wheels => 11,
            Part::Roof => 12,
            Part::Vinyl => 14,
            Part::Paint => 16,
            _ => return false,
        },
        Row::Gauge => 13,
        Row::Decal(_, _) => 15,
        _ => return false,
    };
    model.shop_marker_counts[kind] > 0 && row.enabled(model)
}
