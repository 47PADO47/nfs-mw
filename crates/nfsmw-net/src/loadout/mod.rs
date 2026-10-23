//! Car selection by ids (MP-30, protocol.md section 7.4) and the server's check of it (MP-31).
//!
//! A [`Loadout`] names a car type, parts by slot, a paint, a wheel set and a plate: ids only, never numbers that tune
//! the car, so the server rebuilds the vehicle from its own game data. [`validate`] checks a loadout against a
//! [`Catalog`] the caller implements over its game data and against the session's [`CarRestriction`], and returns a
//! [`LoadoutReject`] whose code goes into `JoinRejected` or a lobby notice. [`Loadout::to_join`] turns a valid loadout
//! into the world's [`nfsmw_sim::PlayerJoin`].

mod restriction;
mod validate;

#[cfg(test)]
mod tests;

pub use restriction::{CarRestriction, MAX_ALLOWED_CARS};
pub use validate::{CarInfo, Catalog, LoadoutReject, validate};

use crate::message::macros::wire_struct;
use crate::message::{List, Plate};

/// Most part choices in one loadout.
pub const MAX_PARTS: usize = 160;
/// Most colour indices in one paint.
pub const MAX_PAINT_COLOURS: usize = 8;

wire_struct! {
    /// One installed part: the slot it fills and the part's id, as the car assembly uses them.
    #[derive(Copy, Eq, Hash)]
    pub struct PartChoice {
        pub slot: u16,
        pub part: u32,
    }
}

wire_struct! {
    /// A paint: its id and the colour indices it takes.
    #[derive(Eq, Hash, Default)]
    pub struct Paint {
        pub id: u32,
        pub colours: List<u8, MAX_PAINT_COLOURS>,
    }
}

wire_struct! {
    /// A player's car choice, ids only.
    #[derive(Eq, Hash, Default)]
    pub struct Loadout {
        /// The car model's hash id.
        pub car_type: u32,
        /// The installed parts, at most one per slot.
        pub parts: List<PartChoice, MAX_PARTS>,
        pub paint: Paint,
        pub wheel_set: u32,
        /// The licence plate text, filtered like any other string.
        pub plate: Plate,
    }
}

impl Loadout {
    /// A loadout of `car_type` with no parts, the default paint and wheels and an empty plate.
    pub fn stock(car_type: u32) -> Self {
        Self { car_type, ..Self::default() }
    }

    /// The world's join request for this loadout: the car type and the part ids in the order given. The spawn point is
    /// the session's to choose.
    pub fn to_join(&self, spawn: Option<[f32; 3]>) -> nfsmw_sim::PlayerJoin {
        nfsmw_sim::PlayerJoin {
            car_type: self.car_type,
            parts: self.parts.iter().map(|choice| choice.part).collect(),
            spawn,
        }
    }
}
