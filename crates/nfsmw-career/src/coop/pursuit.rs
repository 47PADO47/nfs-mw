//! Co-op pursuit settlement: how one shared chase writes into each player's car record
//! (`docs/specs/coop-career.md` section 5). [ours]
//!
//! A co-op chase has one shared result. A bust marks only the car the target was in. An evade runs the evade tick
//! and banks the bounty once, on the lead's car; every other car in the chase takes the infractions and one evade
//! count, but no bounty.

use crate::economy::{PursuitEnd, PursuitResult, RecordedPursuit, end_pursuit_for, record_pursuit};
use crate::stable::Stable;

/// The target car was busted: records a bust on that car only. No evade tick runs and no bounty is banked.
///
/// Returns `None` when the car has no career record, and when the race is a challenge series race (the recorder
/// returns at once, so nothing is recorded).
pub fn settle_target_bust(stable: &mut Stable, handle: u32, result: &PursuitResult) -> Option<RecordedPursuit> {
    if result.challenge_series {
        return None;
    }
    let record = stable.career_for_handle_mut(handle)?;
    let busted = PursuitResult { busted: true, ..*result };
    Some(record_pursuit(record, &busted))
}

/// The shared chase was evaded. The evade tick and the bounty run once, on the banking car `lead`
/// ([`end_pursuit_for`] with `busted` false). Every other car takes the infractions and one evade count, but no
/// bounty. A handle listed twice counts once.
///
/// `lead` is `None` when the banking car has no career record (a guest without one): the evade tick still runs,
/// nothing banks, and the others still take their share.
///
/// A challenge series race records nothing on any car; the evade tick still runs, as in [`end_pursuit_for`].
pub fn settle_evade(stable: &mut Stable, lead: Option<u32>, others: &[u32], result: &PursuitResult) -> PursuitEnd {
    let evaded = PursuitResult { busted: false, ..*result };
    let end = match lead {
        Some(handle) => end_pursuit_for(handle, stable, &evaded),
        None => PursuitEnd { ticked: true, released_cars: stable.evade_tick(), recorded: None },
    };
    if result.challenge_series {
        return end;
    }
    let mut done: Vec<u32> = lead.into_iter().collect();
    for &handle in others {
        if done.contains(&handle) {
            continue;
        }
        done.push(handle);
        let Some(record) = stable.career_for_handle_mut(handle) else {
            continue;
        };
        record.add_unserved(result.infractions);
        record.evaded = record.evaded.saturating_add(1);
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::{InfractionKind, InfractionMask};
    use crate::stable::impound::EVADES_PER_STRIKE_DECAY;
    use crate::stable::{CarRecord, CareerRecord};

    const LEAD: u32 = 0x10;
    const OTHER: u32 = 0x11;

    /// Two owned cars: `LEAD` on career record 0, `OTHER` on career record 1.
    fn two_cars() -> Stable {
        let mut stable = Stable::empty();
        stable.cars[0] =
            CarRecord { handle: LEAD, flags: 0xF_0020 | CarRecord::FLAG_OWNED, career_index: 0, ..CarRecord::free() };
        stable.cars[1] =
            CarRecord { handle: OTHER, flags: 0xF_0020 | CarRecord::FLAG_OWNED, career_index: 1, ..CarRecord::free() };
        stable.careers[0] = CareerRecord::new(0);
        stable.careers[1] = CareerRecord::new(1);
        stable
    }

    fn pursuit(busted: bool) -> PursuitResult {
        PursuitResult {
            busted,
            rep_points_normal: 3000,
            rep_points_destroyed: 500,
            infractions: InfractionMask::of(&[InfractionKind::Speeding, InfractionKind::HitAndRun]),
            challenge_series: false,
        }
    }

    #[test]
    fn bust_marks_only_that_car() {
        let mut stable = two_cars();
        stable.careers[0].decay = 1;
        stable.careers[1].decay = 1;
        let recorded = settle_target_bust(&mut stable, LEAD, &pursuit(false));
        assert_eq!(recorded, Some(RecordedPursuit { banked_bounty: 0, car_bounty: 0, evaded: false }));
        let (lead, other) = (&stable.careers[0], &stable.careers[1]);
        assert_eq!((lead.busted, lead.evaded, lead.bounty), (1, 0, 0));
        assert_eq!(lead.unserved_total(), 2);
        assert_eq!((other.busted, other.evaded, other.bounty, other.unserved_total()), (0, 0, 0, 0));
        assert_eq!((lead.decay, other.decay), (1, 1), "a bust runs no evade tick");
    }

    #[test]
    fn bust_on_a_car_without_a_record_records_nothing() {
        let mut stable = two_cars();
        assert_eq!(settle_target_bust(&mut stable, 0xDEAD, &pursuit(false)), None);
        assert_eq!((stable.careers[0].busted, stable.careers[1].busted), (0, 0));
    }

    #[test]
    fn evade_banks_bounty_once() {
        let mut stable = two_cars();
        let end = settle_evade(&mut stable, Some(LEAD), &[OTHER], &pursuit(false));
        assert!(end.ticked);
        assert_eq!(end.recorded, Some(RecordedPursuit { banked_bounty: 3500, car_bounty: 3500, evaded: true }));
        let (lead, other) = (&stable.careers[0], &stable.careers[1]);
        assert_eq!((lead.bounty, lead.evaded, lead.busted), (3500, 1, 0));
        assert_eq!((other.bounty, other.evaded, other.busted), (0, 1, 0));
        assert_eq!(other.unserved_total(), 2);
    }

    #[test]
    fn evade_ticks_strikes_once() {
        let mut stable = two_cars();
        for record in &mut stable.careers[0..2] {
            record.strikes = 2;
            record.decay = EVADES_PER_STRIKE_DECAY - 1;
        }
        let end = settle_evade(&mut stable, Some(LEAD), &[OTHER], &pursuit(false));
        assert_eq!(end.released_cars, 0);
        for record in &stable.careers[0..2] {
            assert_eq!((record.strikes, record.decay), (1, 0), "one tick per car");
        }
    }

    #[test]
    fn evade_without_a_banking_record_ticks_and_marks_the_others() {
        let mut stable = two_cars();
        stable.careers[0].strikes = 1;
        stable.careers[0].decay = EVADES_PER_STRIKE_DECAY - 1;
        let end = settle_evade(&mut stable, None, &[OTHER], &pursuit(false));
        assert!(end.ticked);
        assert_eq!(end.recorded, None);
        let (lead, other) = (&stable.careers[0], &stable.careers[1]);
        assert_eq!((lead.bounty, lead.evaded, lead.strikes), (0, 0, 0), "one tick, nothing banked");
        assert_eq!((other.bounty, other.evaded, other.unserved_total()), (0, 1, 2));
    }

    #[test]
    fn challenge_series_evade_records_nothing() {
        let mut stable = two_cars();
        let result = PursuitResult { challenge_series: true, ..pursuit(false) };
        let end = settle_evade(&mut stable, Some(LEAD), &[OTHER], &result);
        assert_eq!(end.recorded, None);
        assert_eq!(settle_target_bust(&mut stable, OTHER, &result), None);
        for record in &stable.careers[0..2] {
            assert_eq!((record.bounty, record.evaded, record.busted, record.unserved_total()), (0, 0, 0, 0));
        }
    }
}
