//! Owned Rust values of the career records, and their conversion from and to the ABI records. The career is not built in
//! the port yet, so the host answers unsupported; these types are the shape the calls will return once it is.

use core::mem::size_of;

use blackbox_modding::sdk::{Error, ModResult};

use crate::abi::{
    BbMilestone, BbMilestoneDef, BbName, BbOwnedCar, BbRapSheet, BbRival, BbSafehouse, BbShop, BbShopItem,
    BbShopItemDef, ShopKind,
};

/// The text of an inline name. `Error::Host` when the host wrote bytes that are not UTF-8.
pub(crate) fn name_text(name: &BbName) -> ModResult<String> {
    name.text().map(str::to_owned).ok_or(Error::Host)
}

/// An inline name for a text the mod gives. `Error::Arg` when it is longer than the host accepts.
pub(crate) fn name_abi(text: &str) -> ModResult<BbName> {
    BbName::new(text).ok_or(Error::Arg)
}

/// The rap sheet. [unconfirmed] field list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RapSheet {
    /// Pursuits started.
    pub pursuits: u32,
    /// Times busted.
    pub busted: u32,
    /// Times escaped.
    pub evaded: u32,
    /// Cops wrecked.
    pub cops_wrecked: u32,
    /// Infraction counts, by `Infraction` number.
    pub infractions: [u32; 8],
}

impl RapSheet {
    pub(crate) fn from_abi(raw: &BbRapSheet) -> Self {
        RapSheet {
            pursuits: raw.pursuits,
            busted: raw.busted,
            evaded: raw.evaded,
            cops_wrecked: raw.cops_wrecked,
            infractions: raw.infractions,
        }
    }
}

/// One rival on the blacklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rival {
    /// Rank, 1 to 15.
    pub rank: u32,
    /// The rival's car handle.
    pub car: u64,
    /// Whether the rival is defeated.
    pub defeated: bool,
    /// Whether the rival is locked.
    pub locked: bool,
    /// The rival's name.
    pub name: String,
}

impl Rival {
    pub(crate) fn from_abi(raw: &BbRival) -> ModResult<Self> {
        Ok(Rival {
            rank: raw.rank,
            car: raw.car,
            defeated: raw.defeated != 0,
            locked: raw.locked != 0,
            name: name_text(&raw.name)?,
        })
    }
}

/// One milestone of the career.
#[derive(Debug, Clone, PartialEq)]
pub struct Milestone {
    /// Milestone id.
    pub id: u32,
    /// Progress so far.
    pub progress: f32,
    /// Progress needed.
    pub goal: f32,
    /// Whether it is complete.
    pub done: bool,
    /// Its name.
    pub name: String,
}

impl Milestone {
    pub(crate) fn from_abi(raw: &BbMilestone) -> ModResult<Self> {
        Ok(Milestone {
            id: raw.id,
            progress: raw.progress,
            goal: raw.goal,
            done: raw.done != 0,
            name: name_text(&raw.name)?,
        })
    }
}

/// The definition of a mod milestone to register.
#[derive(Debug, Clone, PartialEq)]
pub struct MilestoneDef {
    /// The registry key, `modid:name`.
    pub key: String,
    /// The name shown to the player.
    pub name: String,
    /// Progress needed.
    pub goal: f32,
    /// Bounty (rep points, not cash) paid on completion.
    pub reward_bounty: u32,
}

impl MilestoneDef {
    pub(crate) fn to_abi(&self) -> ModResult<BbMilestoneDef> {
        Ok(BbMilestoneDef {
            size: size_of::<BbMilestoneDef>() as u32,
            reward_bounty: self.reward_bounty,
            goal: self.goal,
            key: name_abi(&self.key)?,
            name: name_abi(&self.name)?,
        })
    }
}

/// A car the player owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedCar {
    /// The car handle.
    pub car: u64,
    /// Whether it is the car the player drives.
    pub current: bool,
    /// Its name.
    pub name: String,
}

impl OwnedCar {
    pub(crate) fn from_abi(raw: &BbOwnedCar) -> ModResult<Self> {
        Ok(OwnedCar { car: raw.car, current: raw.current != 0, name: name_text(&raw.name)? })
    }
}

/// A safe house.
#[derive(Debug, Clone, PartialEq)]
pub struct Safehouse {
    /// Safe house id.
    pub id: u32,
    /// Whether it is unlocked.
    pub unlocked: bool,
    /// Where it is, render space, metres.
    pub pos: [f32; 3],
    /// Its name.
    pub name: String,
}

impl Safehouse {
    pub(crate) fn from_abi(raw: &BbSafehouse) -> ModResult<Self> {
        Ok(Safehouse { id: raw.id, unlocked: raw.unlocked != 0, pos: raw.pos.to_array(), name: name_text(&raw.name)? })
    }
}

/// A shop.
#[derive(Debug, Clone, PartialEq)]
pub struct Shop {
    /// Shop id.
    pub id: u32,
    /// What it sells.
    pub kind: ShopKind,
    /// Where it is, render space, metres.
    pub pos: [f32; 3],
    /// Its name.
    pub name: String,
}

impl Shop {
    pub(crate) fn from_abi(raw: &BbShop) -> ModResult<Self> {
        Ok(Shop {
            id: raw.id,
            kind: ShopKind::from_raw(raw.kind).ok_or(Error::Host)?,
            pos: raw.pos.to_array(),
            name: name_text(&raw.name)?,
        })
    }
}

/// One item a shop sells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopItem {
    /// Item id.
    pub id: u32,
    /// Price in cash.
    pub price: u32,
    /// The milestone that unlocks it, if any.
    pub unlock: Option<u32>,
    /// Its name.
    pub name: String,
}

impl ShopItem {
    pub(crate) fn from_abi(raw: &BbShopItem) -> ModResult<Self> {
        Ok(ShopItem {
            id: raw.id,
            price: raw.price,
            unlock: (raw.unlock_milestone != 0).then_some(raw.unlock_milestone),
            name: name_text(&raw.name)?,
        })
    }
}

/// The definition of an item to add to a shop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopItemDef {
    /// The registry key, `modid:name`.
    pub key: String,
    /// The name shown to the player.
    pub name: String,
    /// Price in cash.
    pub price: u32,
}

impl ShopItemDef {
    pub(crate) fn to_abi(&self) -> ModResult<BbShopItemDef> {
        Ok(BbShopItemDef {
            size: size_of::<BbShopItemDef>() as u32,
            price: self.price,
            apply: 0,
            key: name_abi(&self.key)?,
            name: name_abi(&self.name)?,
        })
    }
}
