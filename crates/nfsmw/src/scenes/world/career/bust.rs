//! The bust in career mode: markers, cash and impound (`docs/specs/busted-sequence.md` section 10, and
//! `docs/specs/career-economy-flows.md` sections 6.1, 6.2 and 6.5).
//!
//! The police side (`TrafficWorld::pay_bust`) checks the payment and ends the bust. The career side settles the
//! current car's record and the cash once that has succeeded. Outside career mode the bust takes the payment with no
//! markers, as it always did.

use blackbox_pursuit::Infraction;
use nfsmw_career::economy::markers::{MarkerInventory, MarkerKind};
use nfsmw_career::economy::{InfractionKind, InfractionMask, InfractionsScreen, PayOutcome};
use nfsmw_career::settings::CareerSettings;
use nfsmw_career::stable::CareerRecord;

use crate::career::{Career, CareerHandle};
use crate::scenes::world::WorldScene;

impl WorldScene {
    /// The player pays the bust's ticket (`Scene::pay_bust`).
    pub(in crate::scenes::world) fn career_pay_bust(&mut self, marker: bool) -> bool {
        let Some(traffic) = self.traffic.as_mut() else { return false };
        if !self.career.handle.is_active() {
            return traffic.pay_bust(marker, 0);
        }
        let pursuit = pursuit_mask(&traffic.bust_infractions());
        let markers = marker_stock(&self.career.handle);
        if !traffic.pay_bust(marker, markers) {
            return false;
        }
        let outcome = self
            .career
            .handle
            .with(|career| {
                let handle = career.settings.car_handle;
                settle_bust(career, handle, pursuit, marker)
            })
            .flatten();
        log::info!("busted: career settled the bust (marker {marker}), outcome {outcome:?}");
        // The car goes back to the respawn marker when the game runs again (`respawn.rs`).
        self.career.respawn.pending = true;
        true
    }
}

/// The Get Out of Jail Free markers the career holds: the stock a marker payment is checked against. Outside career
/// mode there is no inventory, so the stock is 0 and a marker payment is refused.
fn marker_stock(handle: &CareerHandle) -> u32 {
    handle.with(|career| career.markers.count(MarkerKind::JailFree)).unwrap_or(0)
}

/// The career kind of each bust infraction. The two enums list the same eight kinds in the same order.
pub(super) fn kind_of(infraction: Infraction) -> InfractionKind {
    match infraction {
        Infraction::Speeding => InfractionKind::Speeding,
        Infraction::Racing => InfractionKind::Racing,
        Infraction::RecklessDriving => InfractionKind::RecklessDriving,
        Infraction::Assault => InfractionKind::Assault,
        Infraction::HitAndRun => InfractionKind::HitAndRun,
        Infraction::DamageToProperty => InfractionKind::DamageToProperty,
        Infraction::ResistingArrest => InfractionKind::ResistingArrest,
        Infraction::OffRoad => InfractionKind::OffRoad,
    }
}

/// The set of career kinds the chase committed, from its infractions in any order.
pub(super) fn pursuit_mask(infractions: &[Infraction]) -> InfractionMask {
    let kinds: Vec<InfractionKind> = infractions.iter().map(|infraction| kind_of(*infraction)).collect();
    InfractionMask::of(&kinds)
}

/// Settles one bust payment on the car's record, the cash and the markers. Opening the screen grants the first-visit
/// markers (`markers_to_grant`, 4 when the player holds no Get Out of Jail Free marker). Cash pays every unserved
/// infraction and may impound the car; a marker takes one Get Out of Jail Free marker from the inventory and only this
/// pursuit's infractions off the unserved set. Returns the cash outcome, `None` for a marker.
pub(super) fn settle_payment(
    settings: &mut CareerSettings,
    record: &mut CareerRecord,
    markers: &mut MarkerInventory,
    pursuit: InfractionMask,
    marker: bool,
) -> Option<PayOutcome> {
    let screen = InfractionsScreen::open(settings, record, pursuit, markers.count(MarkerKind::JailFree) > 0);
    markers.add(MarkerKind::JailFree, u32::from(screen.markers_to_grant));
    if marker {
        if !markers.take(MarkerKind::JailFree) {
            log::warn!("busted: a marker payment with no marker in the inventory");
        }
        screen.use_marker(record, pursuit);
        return None;
    }
    Some(screen.pay_with_cash(settings, record))
}

/// Settles the bust on the car with `handle` (the current car, or a co-op player's). Returns `None` when the car has
/// no career record, in which case there are no fines to pay, no strike to add and no markers granted.
pub(super) fn settle_bust(
    career: &mut Career,
    handle: u32,
    pursuit: InfractionMask,
    marker: bool,
) -> Option<PayOutcome> {
    let Some(record) = career.stable.career_for_handle_mut(handle) else {
        log::warn!("busted: the car {handle:#x} has no career record, nothing is fined");
        return None;
    };
    settle_payment(&mut career.settings, record, &mut career.markers, pursuit, marker)
}

#[cfg(test)]
#[path = "bust_tests.rs"]
mod tests;
