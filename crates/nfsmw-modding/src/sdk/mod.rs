//! The SDK a NFS:MW mod links: safe wrappers over the `nfsmw.*` tables, and the typed events (ADR 0005 sections 2 and 8).
//!
//! ```ignore
//! use blackbox_modding::sdk::{Ctx, Mod, ModResult};
//! use nfsmw_modding::sdk::NfsmwExt;
//!
//! struct Heat;
//!
//! impl Mod for Heat {
//!     fn load(ctx: &mut Ctx) -> ModResult<Self> {
//!         if let Some(pursuit) = ctx.nfsmw_pursuit() {
//!             pursuit.set_heat(3.0)?;
//!         }
//!         Ok(Heat)
//!     }
//! }
//! ```
//!
//! The unsafe pointer work lives in `ffi` and nowhere else in this module tree.

mod career;
mod career_types;
mod event;
mod ext;
pub(crate) mod ffi;
mod pursuit;
mod races;

pub use crate::abi::{EndReason, Infraction};
pub use career::Career;
pub use career_types::{Milestone, MilestoneDef, OwnedCar, RapSheet, Rival, Safehouse, Shop, ShopItem, ShopItemDef};
pub use event::MwEvent;
pub use ext::NfsmwExt;
pub use pursuit::{CopRecord, CopTable, EndMode, HeatRow, Pursuit};
pub use races::{Race, RaceDef, RaceResult, RaceState, Races};
