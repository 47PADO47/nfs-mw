//! Original location/position pictures and routing; manufacturer choices share one native icon.
//! Spec: `docs/specs/garage-decals.md`, native navigation icons.

use super::{Item, Model, Page, Row, positions};
use nfsmw_data::car::customization::DecalZone;

mod filter;
pub(in crate::frontend::garage_ui) use filter::{brand, filter, initial_black};

pub(super) const CHOICE_ICON: u32 = 0x697b_4ad4;
const ZONE_ICONS: [u32; 6] = [0x52de_d91d, 0xac79_37b4, 0xda88_b711, 0xc9a9_67c4, 0x2c71_0c4d, 0xffa7_d360];
const LEFT_SLOTS: [u32; 6] = [0xfe95_7f48, 0xfe95_7f49, 0xfe95_7f4a, 0xfe95_7f4b, 0xfe95_7f4c, 0xfe95_7f4d];
const RIGHT_SLOTS: [u32; 6] = [0x2e40_eadb, 0x2e40_eadc, 0x2e40_eadd, 0x2e40_eade, 0x2e40_eadf, 0x2e40_eae0];

pub(super) fn zones(model: &Model, backroom: bool) -> Vec<Item> {
    DecalZone::ALL
        .into_iter()
        .enumerate()
        .map(|(zone, decal)| {
            let count = positions(model, zone);
            let next = match matches!(decal, DecalZone::LeftDoor | DecalZone::RightDoor) || count > 1 {
                true => Page::DecalSlots { zone, backroom },
                false => Page::Choices { row: Row::Decal(zone, 0), backroom },
            };
            Item::page(decal.label(), ZONE_ICONS[zone], next, count > 0)
        })
        .collect()
}

pub(super) fn slots(model: &Model, zone: usize, backroom: bool) -> Vec<Item> {
    (0..positions(model, zone))
        .map(|position| {
            let icon = match DecalZone::ALL[zone] {
                DecalZone::LeftDoor => LEFT_SLOTS.get(position).copied(),
                DecalZone::RightDoor => RIGHT_SLOTS.get(position).copied(),
                _ => None,
            }
            .unwrap_or(ZONE_ICONS[zone]);
            Item::page(
                &format!("Slot {}", position + 1),
                icon,
                Page::Choices { row: Row::Decal(zone, position), backroom },
                true,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
