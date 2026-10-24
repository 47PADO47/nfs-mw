//! Garage checkout over caller-supplied installed prices (`garage-shop.md`).
use super::markers::MarkerInventory;
use crate::settings::{CareerSettings, Flags};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShopError {
    Locked { entity: u32, level: u32 },
    MissingMarker { kind: u32 },
    NotEnoughCash { total: u32, cash: u32 },
    Overflow,
}

impl std::fmt::Display for ShopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Locked { entity, level } => write!(f, "Category {entity}, level {level} is locked"),
            Self::MissingMarker { kind } => write!(f, "A category {kind} backroom marker is required"),
            Self::NotEnoughCash { total, cash } => write!(f, "Not enough cash: ${total} required, ${cash} available"),
            Self::Overflow => write!(f, "The shopping cart total is too large"),
        }
    }
}

impl std::error::Error for ShopError {}

#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub entity: u32,
    pub level: u32,
    pub cost: u32,
    pub trade_in: u32,
    /// Exact original inventory kind and auxiliary key, for a backroom item.
    pub marker: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Quote {
    pub total: u32,
    pub refund: u32,
    pub markers: Vec<(u32, u32)>,
}

pub fn unlocked_level(settings: &CareerSettings, entity: u32, quick_race: bool) -> Option<i8> {
    let offset = usize::try_from(entity).ok()?.checked_mul(8)? + usize::from(quick_race) * 4;
    settings.shop_unlocks.get(offset).map(|value| *value as i8)
}

pub fn quote(settings: &CareerSettings, inventory: &MarkerInventory, items: &[Item]) -> Result<Quote, ShopError> {
    let mut quote = Quote::default();
    let mut available = inventory.clone();
    let mut balance = 0i64;
    for item in items {
        let level = unlocked_level(settings, item.entity, false).unwrap_or(-1);
        let allowance = i64::from(level) + i64::from(item.marker.is_some());
        let complete = settings.flags.contains(Flags::CAREER_COMPLETE);
        if item.level > 0 && i64::from(item.level) > allowance && !complete {
            return Err(ShopError::Locked { entity: item.entity, level: item.level });
        }
        if let Some((kind, aux)) = item.marker {
            if !available.redeem_part(kind, aux) {
                return Err(ShopError::MissingMarker { kind });
            }
            quote.markers.push((kind, aux));
            continue;
        }
        balance = balance.checked_add(i64::from(item.cost) - i64::from(item.trade_in)).ok_or(ShopError::Overflow)?;
    }
    if balance >= 0 {
        quote.total = u32::try_from(balance).map_err(|_| ShopError::Overflow)?;
    }
    if balance < 0 {
        quote.refund = u32::try_from(-balance).map_err(|_| ShopError::Overflow)?;
    }
    if quote.total > settings.cash {
        return Err(ShopError::NotEnoughCash { total: quote.total, cash: settings.cash });
    }
    Ok(quote)
}

/// Revalidates cash and markers before changing either; a refused checkout changes neither.
pub fn checkout(
    settings: &mut CareerSettings,
    inventory: &mut MarkerInventory,
    quote: &Quote,
) -> Result<(), ShopError> {
    let mut remaining = inventory.clone();
    for &(kind, aux) in &quote.markers {
        if !remaining.redeem_part(kind, aux) {
            return Err(ShopError::MissingMarker { kind });
        }
    }
    if quote.total > settings.cash {
        return Err(ShopError::NotEnoughCash { total: quote.total, cash: settings.cash });
    }
    settings.cash -= quote.total;
    settings.add_cash(quote.refund);
    *inventory = remaining;
    Ok(())
}

#[cfg(test)]
#[path = "shop_tests.rs"]
mod tests;
