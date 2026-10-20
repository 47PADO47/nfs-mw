//! What follows the infractions screen, and the career car select it leads to (`docs/specs/busted-aftermath.md`).
//!
//! The payment itself (cash, strikes, impound) is [`super::infractions_screen`]. This module is the screen flow around
//! it: which notice and which animation the car select opens with, what the car select shows for a car, and the
//! release choices for an impounded one. Pure functions; the front end draws the result.

use super::infractions_screen::{PayOutcome, ReleaseError, release_with_cash, release_with_marker};
use super::markers::{MarkerInventory, MarkerKind};
use crate::settings::CareerSettings;
use crate::stable::impound::{MAX_STRIKE_LIMIT, release_fee};
use crate::stable::{CareerRecord, ImpoundState};

/// How the fine was paid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payment {
    Cash,
    Marker,
}

/// The notice shown before the car select when the payment impounded the car.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BustNotice {
    /// `MW_BUSTED_STRIKE_LIMIT`: the strike limit was reached.
    StrikeLimit,
    /// `MW_BUSTED_INSUFFICIENT_FUNDS`: the cash did not cover the fine and was confiscated.
    InsufficientFunds,
}

impl BustNotice {
    /// The language label of the notice text.
    pub const fn label(self) -> &'static str {
        match self {
            BustNotice::StrikeLimit => "MW_BUSTED_STRIKE_LIMIT",
            BustNotice::InsufficientFunds => "MW_BUSTED_INSUFFICIENT_FUNDS",
        }
    }
}

/// The animation the career car select opens with: the argument of its package switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarSelectEntry {
    /// 0: nothing plays (a marker paid, or the screen is opened by the safe house).
    Plain,
    /// `0x100`: the strike that was just added fills its pip.
    NewStrike,
    /// `0x200`: the impounded stamp appears.
    Impounded,
}

impl CarSelectEntry {
    /// The argument the original queues the package with.
    pub const fn arg(self) -> u32 {
        match self {
            CarSelectEntry::Plain => 0,
            CarSelectEntry::NewStrike => 0x100,
            CarSelectEntry::Impounded => 0x200,
        }
    }
}

/// What happens after a payment on the infractions screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BustAftermath {
    /// A notice to dismiss first; the car select opens when it is dismissed.
    pub notice: Option<BustNotice>,
    /// How the car select opens.
    pub entry: CarSelectEntry,
}

/// The aftermath of a payment. `outcome` is the cash payment's result (`None` for a marker, which never impounds).
pub fn after_payment(payment: Payment, outcome: Option<PayOutcome>) -> BustAftermath {
    match (payment, outcome) {
        (Payment::Marker, _) => BustAftermath { notice: None, entry: CarSelectEntry::Plain },
        (Payment::Cash, Some(PayOutcome::StrikeLimit)) => {
            BustAftermath { notice: Some(BustNotice::StrikeLimit), entry: CarSelectEntry::Impounded }
        }
        (Payment::Cash, Some(PayOutcome::InsufficientFunds)) => {
            BustAftermath { notice: Some(BustNotice::InsufficientFunds), entry: CarSelectEntry::Impounded }
        }
        (Payment::Cash, _) => BustAftermath { notice: None, entry: CarSelectEntry::NewStrike },
    }
}

/// The aftermath read back from the car's record after the payment: a record in an impound state means the cash
/// payment impounded it, and which state says why (2 the strike limit, 3 the cash). Used by the front end, which sees
/// the career only after the world settled the bust.
pub fn after_payment_of(payment: Payment, record: Option<&CareerRecord>) -> BustAftermath {
    let outcome = match record.map(CareerRecord::impound_state) {
        Some(ImpoundState::LimitReached) => Some(PayOutcome::StrikeLimit),
        Some(ImpoundState::CashConfiscated) => Some(PayOutcome::InsufficientFunds),
        _ => None,
    };
    after_payment(payment, outcome)
}

/// Where a car stands in the impound, for the car select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarStatus {
    Free,
    /// Impounded and counting down (states 2 and 3).
    Impounded,
    /// The countdown is over (state 4).
    Releasable,
}

/// What the car select shows for one career car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarSelectInfo {
    pub status: CarStatus,
    /// The strike pips: how many are filled and how many the car has (3 to 5).
    pub strikes: u8,
    pub strike_limit: u8,
    /// The car's heat (the meter).
    pub heat: f32,
    /// A free car shows its bounty and its unserved fines; an impounded one shows the release cost.
    pub bounty: u32,
    pub fines: u64,
    pub release_cost: u32,
}

/// The car select's numbers for a car of price `cost`.
pub fn car_select_info(record: &CareerRecord, cost: u32) -> CarSelectInfo {
    let status = match record.impound_state() {
        ImpoundState::Free => CarStatus::Free,
        ImpoundState::Releasable => CarStatus::Releasable,
        _ => CarStatus::Impounded,
    };
    CarSelectInfo {
        status,
        strikes: record.strikes.clamp(0, MAX_STRIKE_LIMIT) as u8,
        strike_limit: record.strike_limit.clamp(1, MAX_STRIKE_LIMIT) as u8,
        heat: record.heat,
        bounty: record.bounty,
        fines: super::infraction::fine_of_counters(&record.unserved),
        release_cost: release_fee(cost),
    }
}

/// Leaving the car select with Back is refused while the car the player came with is still impounded or waiting for
/// its release: the player has to choose another car (`CS_IMPOUND_MUST_CHOOSE_NEW_CAR`).
pub fn back_is_blocked(original: &CareerRecord) -> bool {
    original.impound_state() != ImpoundState::Free
}

/// The ways to release an impounded car the player can choose from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseWay {
    Cash,
    Marker,
}

/// The release dialog for an impounded or releasable car (`MaybeReleaseCar`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseDialog {
    /// Neither the cash (or the car is still counting down) nor a marker: a notice says it cannot be released.
    Cannot,
    /// One way, or both, and cancel.
    Choose { cash: bool, marker: bool },
}

/// What the player can do with an impounded car: pay the fee when the car is releasable and the cash covers it, use an
/// Impound Release marker when one is held (at any time while impounded).
pub fn release_dialog(
    record: &CareerRecord,
    settings: &CareerSettings,
    cost: u32,
    markers: &MarkerInventory,
) -> ReleaseDialog {
    let cash = record.is_releasable() && settings.can_afford(release_fee(cost));
    let marker = markers.count(MarkerKind::ImpoundRelease) > 0;
    match cash || marker {
        true => ReleaseDialog::Choose { cash, marker },
        false => ReleaseDialog::Cannot,
    }
}

/// Releases the car the chosen way. The cash way needs a releasable car and the fee; the marker way consumes an Impound
/// Release marker and works while the car is still impounded (`release_with_marker`). Heat goes to 0 either way.
pub fn release(
    way: ReleaseWay,
    settings: &mut CareerSettings,
    record: &mut CareerRecord,
    cost: u32,
    markers: &mut MarkerInventory,
) -> Result<(), ReleaseError> {
    match way {
        ReleaseWay::Cash => release_with_cash(settings, record, cost).map(|_| ()),
        ReleaseWay::Marker => {
            if markers.count(MarkerKind::ImpoundRelease) == 0 || !release_with_marker(record) {
                return Err(ReleaseError::NotReleasable);
            }
            markers.take(MarkerKind::ImpoundRelease);
            Ok(())
        }
    }
}

/// What the Extra Impound Strike marker offers on the car select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrikeMarker {
    /// A marker is held and the free car's limit is below 5: ask to use it.
    Offer,
    /// The limit is already 5.
    AtMaximum,
    /// Nothing to offer (no marker, or the car is impounded).
    Nothing,
}

/// The Extra Impound Strike choice for a car (`MaybeAddImpoundBox`).
pub fn strike_marker(record: &CareerRecord, markers: &MarkerInventory) -> StrikeMarker {
    let can_add = record.is_free_to_drive() && record.strike_limit < MAX_STRIKE_LIMIT;
    if can_add && markers.count(MarkerKind::ExtraStrike) > 0 {
        return StrikeMarker::Offer;
    }
    if record.strike_limit >= MAX_STRIKE_LIMIT {
        return StrikeMarker::AtMaximum;
    }
    StrikeMarker::Nothing
}

/// Uses an Extra Impound Strike marker on the car: the limit goes up by one and the marker is consumed. False and
/// nothing consumed when the marker does not apply.
pub fn use_strike_marker(record: &mut CareerRecord, markers: &mut MarkerInventory) -> bool {
    if markers.count(MarkerKind::ExtraStrike) == 0 || !record.apply_extra_strike_marker() {
        return false;
    }
    markers.take(MarkerKind::ExtraStrike)
}

#[cfg(test)]
#[path = "aftermath_tests.rs"]
mod tests;
