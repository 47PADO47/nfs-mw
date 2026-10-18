//! `nfsmw.career` on the game side: the backend of the career being played, read and written through the shared
//! [`CareerHandle`]. Outside career mode (no career loaded, quick race, free roam from the command line) every call
//! answers `BB_ERR_NOT_READY`, as the world calls do with no world loaded. The table is offered either way, so a mod
//! finds it.
//!
//! What the calls do (the rules are `nfsmw-career`; the contract is `docs/modding/api-nfsmw.md`):
//! - Cash: `cash_add` with a negative delta spends, clamped at 0 as the original does. `cash_set` takes 0 to `u32::MAX`.
//! - Bounty: the player's bounty is the sum over the stable (the sold base plus every live car). `bounty_add` and
//!   `bounty_set` change the current car's career record; `bounty_set` answers `BB_ERR_ARG` for a total below the
//!   other cars' share. Both give the new total to the Blacklist, which re-checks the rival gates.
//! - Rap sheet: the figures of `RapTotals`. `rap_sheet_set_stat` and `rap_sheet_add_stat` change the lifetime sums of
//!   statistics 0 to 7; statistics 8 and 9 are computed, so they answer `BB_ERR_ARG`.
//! - Blacklist: 15 rivals, one per bin 1 to 15, with rank = bin number. `rival_at` lists them in rank order: index 0
//!   is rank 1. The rival's car is handle 0 and its name is `Rank N`: the bin data names neither
//!   **[unconfirmed]**.
//! - Milestones: the id is the 1 based position in the career's list. `milestone_complete` banks a milestone that is
//!   not locked, pays its bounty to the current car and counts it as a challenge of its bin. `milestone_progress` is
//!   not offered: the progress of a career milestone is the game's statistic, not a value a mod sets.
//! - Stage, the garage (`garage_count`, `garage_at`), the current car (`current_car_get`, `current_car_set`).
//!
//! Stays `BB_ERR_UNSUPPORTED` (the trait defaults), because the state behind it does not exist yet:
//! `milestone_register`, `milestone_progress`, `rival_defeat` and `rival_set_locked` (the Blacklist exposes no setter
//! for a rival's state), `give_car`, `remove_car`, `save`, `slot_get` (the career keeps no save slot), every safe house
//! call and every shop call.

use nfsmw_career::blacklist::BinState;
use nfsmw_career::milestones::{MilestoneSet, MilestoneState};
use nfsmw_career::rapsheet::RapTotals;
use nfsmw_career::stable::{CarRecord, CareerRecord, FREE_HANDLE, InfractionSet};
use nfsmw_modding::abi::{BbMilestone, BbName, BbOwnedCar, BbRapSheet, BbRival};
use nfsmw_modding::host::{BackendResult, CareerBackend};

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_DENIED, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY};

use crate::career::{Career, CareerHandle};

/// The Blacklist's rivals: bins 1 to 15.
const RIVALS: u32 = 15;

/// The `nfsmw.career` port of the game: one per mod table, reading and writing the career behind `handle`.
pub struct CareerPort {
    handle: CareerHandle,
}

impl CareerPort {
    /// The port over the career `handle`. The game passes [`CareerHandle::shared`].
    pub fn new(handle: CareerHandle) -> CareerPort {
        CareerPort { handle }
    }

    /// Runs `f` on the career being played, or answers `BB_ERR_NOT_READY` outside career mode.
    fn with<R>(&self, f: impl FnOnce(&mut Career) -> BackendResult<R>) -> BackendResult<R> {
        match self.handle.with(f) {
            Some(result) => result,
            None => Err(BB_ERR_NOT_READY),
        }
    }
}

/// A value as a `u32`, clamped at `u32::MAX`.
fn clamp_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// `index` when it is below `len`, else `BB_ERR_NOT_FOUND`.
fn in_range(len: usize, index: u32) -> BackendResult<usize> {
    let index = usize::try_from(index).map_err(|_| BB_ERR_NOT_FOUND)?;
    match index < len {
        true => Ok(index),
        false => Err(BB_ERR_NOT_FOUND),
    }
}

/// The index of the milestone with 1 based `id`, when it is in the list.
fn milestone_slot(len: usize, id: u32) -> BackendResult<usize> {
    let index = id.checked_sub(1).ok_or(BB_ERR_NOT_FOUND)?;
    in_range(len, index)
}

/// The rank of the rival at `index` (index 0 is rank 1).
fn rank_at(index: u32) -> BackendResult<u8> {
    let index = in_range(RIVALS as usize, index)?;
    Ok(index as u8 + 1)
}

/// The rap sheet statistic that is stored: 0 to 7. Statistics 8 and 9 are computed from the others.
fn stored_stat(stat: u32) -> BackendResult<usize> {
    match stat {
        0..=7 => Ok(stat as usize),
        _ => Err(BB_ERR_ARG),
    }
}

/// The career record of the car the player drives, or `BB_ERR_NOT_FOUND` when no car is driven.
fn current_record(career: &mut Career) -> BackendResult<&mut CareerRecord> {
    let handle = career.settings.car_handle;
    career.stable.career_for_handle_mut(handle).ok_or(BB_ERR_NOT_FOUND)
}

/// Gives the Blacklist the player's bounty total. Its rival gates re-check on it.
fn sync_bounty(career: &mut Career) {
    let total = clamp_u32(career.stable.total_bounty());
    let _ = career.blacklist.bounty_changed(total);
}

/// The cars the garage lists: every car record with a handle, in slot order.
fn owned_cars(career: &Career) -> impl Iterator<Item = &CarRecord> {
    career.stable.cars.iter().filter(|car| car.handle != FREE_HANDLE)
}

/// A name copied inline; empty when it does not fit.
fn name_of(text: &str) -> BbName {
    BbName::new(text).unwrap_or(BbName::EMPTY)
}

impl CareerBackend for CareerPort {
    fn cash_get(&self, _owner: &str) -> BackendResult<i64> {
        self.with(|career| Ok(i64::from(career.cash())))
    }

    fn cash_add(&self, _owner: &str, delta: i64, _reason: &str) -> BackendResult<()> {
        self.with(|career| {
            let amount = clamp_u32(delta.unsigned_abs());
            if delta >= 0 {
                career.settings.add_cash(amount);
                return Ok(());
            }
            career.settings.spend_cash(amount);
            Ok(())
        })
    }

    fn cash_set(&self, _owner: &str, value: i64) -> BackendResult<()> {
        let cash = u32::try_from(value).map_err(|_| BB_ERR_ARG)?;
        self.with(|career| {
            career.settings.cash = cash;
            Ok(())
        })
    }

    fn bounty_get(&self, _owner: &str) -> BackendResult<u64> {
        self.with(|career| Ok(career.stable.total_bounty()))
    }

    fn bounty_add(&self, _owner: &str, delta: u64) -> BackendResult<()> {
        self.with(|career| {
            let record = current_record(career)?;
            record.bounty = clamp_u32(u64::from(record.bounty).saturating_add(delta));
            sync_bounty(career);
            Ok(())
        })
    }

    fn bounty_set(&self, _owner: &str, value: u64) -> BackendResult<()> {
        self.with(|career| {
            let total = career.stable.total_bounty();
            let record = current_record(career)?;
            let others = total.saturating_sub(u64::from(record.bounty));
            let own = value.checked_sub(others).ok_or(BB_ERR_ARG)?;
            record.bounty = u32::try_from(own).map_err(|_| BB_ERR_ARG)?;
            sync_bounty(career);
            Ok(())
        })
    }

    fn rap_sheet_get(&self, _owner: &str) -> BackendResult<BbRapSheet> {
        self.with(|career| {
            let totals = RapTotals::compute(&career.stable, &career.history);
            let unserved = career.stable.infraction_totals(InfractionSet::Unserved);
            let served = career.stable.infraction_totals(InfractionSet::Served);
            Ok(BbRapSheet {
                size: 0,
                pursuits: totals.times_evaded.saturating_add(totals.times_busted),
                busted: totals.times_busted,
                evaded: totals.times_evaded,
                cops_wrecked: totals.police_destroyed,
                infractions: std::array::from_fn(|kind| unserved[kind].saturating_add(served[kind])),
            })
        })
    }

    fn rap_sheet_set_stat(&self, _owner: &str, stat: u32, value: u64) -> BackendResult<()> {
        let slot = stored_stat(stat)?;
        self.with(|career| {
            career.history.sums[slot] = clamp_u32(value);
            Ok(())
        })
    }

    fn rap_sheet_add_stat(&self, _owner: &str, stat: u32, delta: u64) -> BackendResult<()> {
        let slot = stored_stat(stat)?;
        self.with(|career| {
            let sum = &mut career.history.sums[slot];
            *sum = clamp_u32(u64::from(*sum).saturating_add(delta));
            Ok(())
        })
    }

    fn rival_count(&self, _owner: &str) -> BackendResult<u32> {
        self.with(|_| Ok(RIVALS))
    }

    fn rival_at(&self, _owner: &str, index: u32) -> BackendResult<BbRival> {
        self.with(|career| {
            let rank = rank_at(index)?;
            let bin = career.blacklist.bin(rank).ok_or(BB_ERR_NOT_FOUND)?;
            Ok(BbRival {
                size: 0,
                rank: u32::from(rank),
                car: 0,
                defeated: u8::from(bin.state == Some(BinState::Done)),
                locked: u8::from(matches!(bin.state, None | Some(BinState::RivalLocked))),
                _pad: [0; 2],
                name: name_of(&format!("Rank {rank}")),
            })
        })
    }

    fn milestone_count(&self, _owner: &str) -> BackendResult<u32> {
        self.with(|career| Ok(clamp_u32(career.milestones.milestones().len() as u64)))
    }

    fn milestone_at(&self, _owner: &str, index: u32) -> BackendResult<BbMilestone> {
        self.with(|career| {
            let slot = in_range(career.milestones.milestones().len(), index)?;
            let milestone = career.milestones.milestones()[slot];
            Ok(BbMilestone {
                size: 0,
                id: clamp_u32(slot as u64 + 1),
                progress: career.milestones.stat(milestone.def.stat).current.unwrap_or(0.0),
                goal: milestone.def.threshold,
                done: u8::from(milestone.state == MilestoneState::Complete),
                _pad: [0; 3],
                name: name_of(&format!("milestone {:#x}", milestone.def.key)),
            })
        })
    }

    fn milestone_complete(&self, _owner: &str, id: u32) -> BackendResult<()> {
        self.with(|career| {
            let mut list = career.milestones.milestones().to_vec();
            let slot = milestone_slot(list.len(), id)?;
            let milestone = list[slot];
            match milestone.state {
                MilestoneState::Complete => return Ok(()),
                MilestoneState::Locked => return Err(BB_ERR_DENIED),
                MilestoneState::Active | MilestoneState::AtStake => {}
            }
            let record = current_record(career)?;
            record.bounty = clamp_u32(u64::from(record.bounty).saturating_add(u64::from(milestone.def.bounty)));
            list[slot].state = MilestoneState::Complete;
            let stats = *career.milestones.stats();
            let summary = career.milestones.pursuit_summary().to_vec();
            career.milestones = MilestoneSet::from_parts(list, stats, summary);
            let _ = career.blacklist.challenge_passed(milestone.def.bin, &milestone.def.key.to_string());
            sync_bounty(career);
            Ok(())
        })
    }

    fn stage_get(&self, _owner: &str) -> BackendResult<u32> {
        self.with(|career| Ok(u32::from(career.stage())))
    }

    fn garage_count(&self, _owner: &str) -> BackendResult<u32> {
        self.with(|career| Ok(clamp_u32(owned_cars(career).count() as u64)))
    }

    fn garage_at(&self, _owner: &str, index: u32) -> BackendResult<BbOwnedCar> {
        self.with(|career| {
            let slot = in_range(owned_cars(career).count(), index)?;
            let car = owned_cars(career).nth(slot).ok_or(BB_ERR_NOT_FOUND)?;
            let name = career
                .data
                .cars
                .iter()
                .find(|lot| lot.vehicle_key == car.vehicle_key)
                .map_or_else(String::new, |lot| lot.name.clone());
            Ok(BbOwnedCar {
                size: 0,
                _pad: 0,
                car: u64::from(car.handle),
                current: u8::from(car.handle == career.settings.car_handle),
                _pad2: [0; 3],
                name: name_of(&name),
            })
        })
    }

    fn current_car_get(&self, _owner: &str) -> BackendResult<u64> {
        self.with(|career| {
            let handle = career.settings.car_handle;
            Ok(career.stable.car_by_handle(handle).map_or(0, |_| u64::from(handle)))
        })
    }

    fn current_car_set(&self, _owner: &str, car: u64) -> BackendResult<()> {
        self.with(|career| {
            let handle = u32::try_from(car).map_err(|_| BB_ERR_NOT_FOUND)?;
            if career.stable.car_by_handle(handle).is_none() {
                return Err(BB_ERR_NOT_FOUND);
            }
            career.settings.car_handle = handle;
            Ok(())
        })
    }
}
