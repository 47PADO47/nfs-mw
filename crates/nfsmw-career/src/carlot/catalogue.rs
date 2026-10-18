//! The catalogue entries and their unlock rules (`docs/specs/career-car-lot.md` sections 1, 2 and 6).
//!
//! [`LotCar`] is this crate's own copy of one front-end collection: the caller reads `FE_ATTRIB.bin` and fills
//! the fields, so nothing here depends on `nfsmw-data`. The file stores the price as an `i32`; this crate keeps
//! a `u32` and the caller clamps a negative value to 0 (unconfirmed: the game's compare with a negative price is
//! not checked).

use crate::settings::Flags;

/// Low-half flag of a car that is in the car lot (spec section 1, `0x1`).
pub const FLAG_IN_LOT: u32 = 0x1;
/// Low-half flag of a car that is not for sale (spec section 1, `0x20`).
pub const FLAG_NOT_FOR_SALE: u32 = 0x20;

/// The class of a car (layout offset `0x38`, spec section 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotClass {
    /// 0: North America.
    NorthAmerica,
    /// 1: Europe.
    Europe,
    /// 2: Japan.
    Japan,
    /// 3: the root "vehicles" class (traffic, cops, specials). Never for sale.
    Vehicles,
    /// Any other value. The game does not produce it; this crate never offers such a car (unconfirmed).
    Other(u32),
}

impl LotClass {
    /// The class of a stored value.
    pub const fn from_value(value: u32) -> LotClass {
        match value {
            0 => LotClass::NorthAmerica,
            1 => LotClass::Europe,
            2 => LotClass::Japan,
            3 => LotClass::Vehicles,
            other => LotClass::Other(other),
        }
    }

    /// The stored value of the class.
    pub const fn value(self) -> u32 {
        match self {
            LotClass::NorthAmerica => 0,
            LotClass::Europe => 1,
            LotClass::Japan => 2,
            LotClass::Vehicles => 3,
            LotClass::Other(value) => value,
        }
    }

    /// The high-half class bit of a car record: `0x10000 << class` for classes 0 to 3 (spec section 1, **[exe]**).
    /// `None` for any other class.
    pub const fn flag_bit(self) -> Option<u32> {
        match self {
            LotClass::Other(_) => None,
            _ => Some(0x1_0000 << self.value()),
        }
    }
}

/// One car of the career car lot: the front-end collection fields the rules read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotCar {
    /// The collection name, e.g. `cobaltss`. For the UI and logs only; no rule reads it.
    pub name: String,
    /// The car handle a bought car gets. **[unconfirmed]** the spec copies the lot record into a free slot but does
    /// not say whether the lot's handle or a new handle is used; the caller supplies it.
    pub handle: u32,
    /// The front-end collection key the car record's `frontend_key` points at (spec section 1).
    pub frontend_key: u32,
    /// The vehicle collection key the car record's `vehicle_key` points at.
    pub vehicle_key: u32,
    /// The price in dollars (layout `0x4C`, `0xE61673E9`).
    pub price: u32,
    /// The unlock rank (layout `0x59`, `0x60F99F57`): available while the stage is at most this value.
    pub unlock_rank: u8,
    /// The for-sale flag (layout `0x58`, `0xFBCF1A48`).
    pub for_sale: bool,
    /// The class (layout `0x38`).
    pub class: LotClass,
}

/// How a catalogue car shows in the car lot at a stage (spec section 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotState {
    /// Available at this stage and for sale: can be bought.
    Buyable,
    /// Listed but greyed: its unlock rank is below the stage.
    Locked,
    /// Not offered in the lot at all (class 3, or the for-sale flag is false).
    NotOffered,
}

impl LotCar {
    /// True when the car is available at `stage`: its unlock rank is at least the stage (spec section 1, **[exe]**).
    /// A rank 16 car is available from the prologue on; a rank 14 car once rival 15 is beaten (stage 14).
    pub const fn is_available(&self, stage: u8) -> bool {
        stage <= self.unlock_rank
    }

    /// True when the car is offered in the lot: the for-sale flag is set and the class is one of the three car
    /// classes, not the vehicles class (spec section 1, **[exe]** for the flag rule).
    pub fn is_offered(&self) -> bool {
        self.for_sale && !matches!(self.class, LotClass::Vehicles | LotClass::Other(_))
    }

    /// The car's state in the lot at `stage`.
    pub fn state_at(&self, stage: u8) -> LotState {
        if !self.is_offered() {
            return LotState::NotOffered;
        }
        if self.is_available(stage) {
            return LotState::Buyable;
        }
        LotState::Locked
    }

    /// The low and high flag bits a new car record gets from this entry, before the owned flag is added
    /// (spec section 1, **[exe]**): the class bit in the high half; `0x20` when the class is 3 or the car is not
    /// for sale, else `0x1`.
    pub fn lot_flags(&self) -> u32 {
        let class_bit = self.class.flag_bit().unwrap_or(0);
        let low = if self.is_offered() { FLAG_IN_LOT } else { FLAG_NOT_FOR_SALE };
        class_bit | low
    }
}

/// The offered cars in the lot at `stage` with their states, in catalogue order. Locked cars are listed (spec 1);
/// cars that are not offered are left out.
pub fn listed(catalogue: &[LotCar], stage: u8) -> Vec<(&LotCar, LotState)> {
    catalogue.iter().filter(|car| car.is_offered()).map(|car| (car, car.state_at(stage))).collect()
}

/// The catalogue entry whose front-end key is `key`, if any.
pub fn by_frontend_key(catalogue: &[LotCar], key: u32) -> Option<&LotCar> {
    catalogue.iter().find(|car| car.frontend_key == key)
}

/// True when a Blacklist rival's preset car is available: rival `rank` (`BL2` to `BL15`) is available when the
/// stage is below the rank, that is after rival `rank` is beaten (spec section 6, **[exe]**). Rank 1 is not in
/// that list and every rank outside 2 to 15 returns false.
pub const fn rival_car_available(rank: u8, stage: u8) -> bool {
    if rank < 2 || rank > 15 {
        return false;
    }
    stage < rank
}

/// The career flag a bonus preset car needs, by its handle (spec section 6, **[exe]**). `None` for a handle
/// that is not in this table.
///
/// The spec gives bit 14 for both `BONUS_C6R` and the E3 demo BMW, though its messages name a different unlock
/// for each (100 percent completion, winning the career). Both are mapped to bit 14 as the spec says; the
/// conflict is an open question.
pub fn bonus_flag(handle: u32) -> Option<Flags> {
    match handle {
        // BONUS_C6R, E3_DEMO_BMW: bit 14 (`0x4000`, the career-complete bit in `settings::Flags`).
        0x2CF3_70F0 | 0x03A9_4520 => Some(Flags::CAREER_COMPLETE),
        // BONUS_SL65: bit 15 (`0x8000`).
        0xCB6A_AF2F => Some(Flags::EVENT_19_8_69_DONE),
        // BONUS_GT2: bit 16 (`0x10000`).
        0x2CF3_85B2 => Some(Flags::CHALLENGE_SERIES_DONE),
        // The unnamed preset: bit 18 (`0x40000`).
        0x3449_8EB2 => Some(Flags::BONUS_CAR_0X34498EB2),
        _ => None,
    }
}

/// Whether a bonus preset is unlocked by `flags`. `None` when `handle` is not a bonus preset (see [`bonus_flag`]).
pub fn bonus_unlocked(handle: u32, flags: Flags) -> Option<bool> {
    let needed = bonus_flag(handle)?;
    Some(flags.contains(needed))
}
