//! Records of the `nfsmw.career` interface. Each is an out or in record, size-prefixed: `size` is the number of bytes the
//! writer filled in (an out record: the caller sets it to its own size before the call). Names are inline [`BbName`]s,
//! so no record holds a pointer. Field lists are [unconfirmed] against the career spec until the career lands.

use super::shared::{BbName, BbVec3};

/// The rap sheet: the player's record of pursuits and infractions. [unconfirmed] field list.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbRapSheet {
    /// Bytes the host filled in.
    pub size: u32,
    /// Pursuits started.
    pub pursuits: u32,
    /// Times busted.
    pub busted: u32,
    /// Times escaped.
    pub evaded: u32,
    /// Cops wrecked.
    pub cops_wrecked: u32,
    /// Infraction counts, indexed by the `Infraction` number.
    pub infractions: [u32; 8],
}

/// One of the rivals of the blacklist.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbRival {
    /// Bytes the host filled in.
    pub size: u32,
    /// Rank on the blacklist, 1 to 15.
    pub rank: u32,
    /// The rival's car handle (the car id the host registers).
    pub car: u64,
    /// Non-zero once the rival is defeated.
    pub defeated: u8,
    /// Non-zero while the rival is locked.
    pub locked: u8,
    /// Padding, zero.
    pub _pad: [u8; 2],
    /// The rival's name.
    pub name: BbName,
}

/// A milestone of the career, as the host reports it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbMilestone {
    /// Bytes the host filled in.
    pub size: u32,
    /// Milestone id.
    pub id: u32,
    /// Progress so far, in the units of `goal`.
    pub progress: f32,
    /// Progress needed.
    pub goal: f32,
    /// Non-zero once complete.
    pub done: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
    /// The milestone's name.
    pub name: BbName,
}

/// The definition a mod gives to register a milestone. Size-prefixed: the host reads as many bytes as the size says.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbMilestoneDef {
    /// Bytes the mod filled in.
    pub size: u32,
    /// Cash paid when the milestone completes.
    pub reward_cash: u32,
    /// Progress needed.
    pub goal: f32,
    /// The registry key, `modid:name`.
    pub key: BbName,
    /// The name shown to the player.
    pub name: BbName,
}

/// A car the player owns.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbOwnedCar {
    /// Bytes the host filled in.
    pub size: u32,
    /// Padding, zero.
    pub _pad: u32,
    /// The car handle.
    pub car: u64,
    /// Non-zero for the car the player drives now.
    pub current: u8,
    /// Padding, zero.
    pub _pad2: [u8; 3],
    /// The car's name.
    pub name: BbName,
}

/// A safe house.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbSafehouse {
    /// Bytes the host filled in.
    pub size: u32,
    /// Safe house id.
    pub id: u32,
    /// Non-zero once unlocked.
    pub unlocked: u8,
    /// Padding, zero.
    pub _pad: [u8; 3],
    /// Where it is, render space, metres.
    pub pos: BbVec3,
    /// Its name.
    pub name: BbName,
}

/// A shop: auto shop, car lot, performance or paint.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BbShop {
    /// Bytes the host filled in.
    pub size: u32,
    /// Shop id.
    pub id: u32,
    /// `ShopKind` number.
    pub kind: u32,
    /// Where it is, render space, metres.
    pub pos: BbVec3,
    /// Its name.
    pub name: BbName,
}

/// One item a shop sells.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbShopItem {
    /// Bytes the host filled in.
    pub size: u32,
    /// Item id.
    pub id: u32,
    /// Price in cash.
    pub price: u32,
    /// Milestone id that unlocks the item, or 0 when none does. Milestone ids start at 1.
    pub unlock_milestone: u32,
    /// The item's name.
    pub name: BbName,
}

/// The definition a mod gives to add an item to a shop. Size-prefixed.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BbShopItemDef {
    /// Bytes the mod filled in.
    pub size: u32,
    /// Price in cash.
    pub price: u32,
    /// Host callback id that applies the item, or 0 for none. Reserved: the host answers unsupported until callbacks exist.
    pub apply: u32,
    /// The registry key, `modid:name`.
    pub key: BbName,
    /// The name shown to the player.
    pub name: BbName,
}
