//! A player pursuit ends: the order of events and what the recorder writes
//! (`docs/specs/career-economy-flows.md` section 2).

use super::infraction::InfractionMask;
use crate::settings::CareerSettings;
use crate::stable::{CareerRecord, Stable};

/// The numbers of a finished pursuit that the career cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PursuitResult {
    /// The pursuit ended in a bust. The spec marks the meaning of the original flag **[unconfirmed]** and says to
    /// model it as "the pursuit ended in a bust".
    pub busted: bool,
    /// Rep points of the perpetrator from normal play (interface slot `+0x44`).
    pub rep_points_normal: u32,
    /// Rep points from destruction (slot `+0x40`).
    pub rep_points_destroyed: u32,
    /// The infraction kinds committed (the manager mask).
    pub infractions: InfractionMask,
    /// The current race event is a challenge series race: the recorder returns at once.
    pub challenge_series: bool,
}

impl PursuitResult {
    /// The bounty of the pursuit: the two rep point fields summed (saturating).
    pub fn bounty(&self) -> u32 {
        self.rep_points_normal.saturating_add(self.rep_points_destroyed)
    }
}

/// What the recorder wrote into one car record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedPursuit {
    /// The bounty added to the car (0 for a bust: a busted pursuit banks nothing).
    pub banked_bounty: u32,
    /// The car bounty after the pursuit.
    pub car_bounty: u32,
    /// True when the evaded count rose, false when the busted count rose.
    pub evaded: bool,
}

/// Step 2.6 of the recorder on one record: the infraction mask goes into the **unserved** counters for evaded
/// and busted pursuits alike; then, if the pursuit was not busted, `evaded += 1` and `bounty += bounty`;
/// otherwise `busted += 1` and nothing else. Counters saturate.
pub fn record_pursuit(record: &mut CareerRecord, result: &PursuitResult) -> RecordedPursuit {
    record.add_unserved(result.infractions);
    if result.busted {
        record.busted = record.busted.saturating_add(1);
        return RecordedPursuit { banked_bounty: 0, car_bounty: record.bounty, evaded: false };
    }
    let bounty = result.bounty();
    record.evaded = record.evaded.saturating_add(1);
    record.bounty = record.bounty.saturating_add(bounty);
    RecordedPursuit { banked_bounty: bounty, car_bounty: record.bounty, evaded: true }
}

/// What a finished pursuit did to the career.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PursuitEnd {
    /// The evade ticker ran (the pursuit was not busted).
    pub ticked: bool,
    /// How many impounded cars the evade tick made releasable.
    pub released_cars: usize,
    /// What the recorder wrote; `None` when it returned early (a challenge series race) or the current car has
    /// no career record.
    pub recorded: Option<RecordedPursuit>,
}

/// A player pursuit is removed, in the order of the spec:
///
/// 1. the listener, for a pursuit that was not busted: the strike ticker for player 0 runs over the stable
///    (the heat factor, the script variables and the gameplay-manager notice are the caller's);
/// 2. the recorder: returns at once for a challenge series race, otherwise finds the current car record
///    (`settings.car_handle`) and applies [`record_pursuit`].
///
/// The profile pursuit history, the HUD notice and the bounty broadcast are left to the caller.
pub fn end_player_pursuit(settings: &CareerSettings, stable: &mut Stable, result: &PursuitResult) -> PursuitEnd {
    end_pursuit_for(settings.car_handle, stable, result)
}

/// [`end_player_pursuit`] for the car with this handle instead of the current car. The evade tick runs over the
/// whole stable, as before; only the recorder's car changes.
pub fn end_pursuit_for(handle: u32, stable: &mut Stable, result: &PursuitResult) -> PursuitEnd {
    let released_cars = if result.busted { 0 } else { stable.evade_tick() };
    let mut end = PursuitEnd { ticked: !result.busted, released_cars, recorded: None };
    if result.challenge_series {
        return end;
    }
    let Some(record) = stable.career_for_handle_mut(handle) else {
        return end;
    };
    end.recorded = Some(record_pursuit(record, result));
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::InfractionKind;
    use crate::stable::CarRecord;

    fn setup() -> (CareerSettings, Stable) {
        let settings = CareerSettings::new_career();
        let mut stable = Stable::empty();
        stable.cars[0] = CarRecord {
            handle: settings.car_handle,
            flags: 0xF_0020 | CarRecord::FLAG_OWNED,
            career_index: 0,
            ..CarRecord::free()
        };
        stable.careers[0] = CareerRecord::new(0);
        (settings, stable)
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
    fn an_evaded_pursuit_banks_the_summed_rep_points() {
        let (settings, mut stable) = setup();
        let end = end_player_pursuit(&settings, &mut stable, &pursuit(false));
        let recorded = end.recorded.unwrap();
        assert_eq!(recorded, RecordedPursuit { banked_bounty: 3500, car_bounty: 3500, evaded: true });
        assert!(end.ticked);
        let r = &stable.careers[0];
        assert_eq!((r.bounty, r.evaded, r.busted), (3500, 1, 0));
        assert_eq!(r.unserved, [1, 0, 0, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn a_busted_pursuit_adds_infractions_and_a_bust_but_no_bounty() {
        let (settings, mut stable) = setup();
        let end = end_player_pursuit(&settings, &mut stable, &pursuit(true));
        assert!(!end.ticked);
        assert_eq!(end.recorded, Some(RecordedPursuit { banked_bounty: 0, car_bounty: 0, evaded: false }));
        let r = &stable.careers[0];
        assert_eq!((r.bounty, r.evaded, r.busted), (0, 0, 1));
        assert_eq!(r.unserved_total(), 2);
        assert_eq!(r.served_total(), 0);
    }

    #[test]
    fn a_challenge_series_pursuit_never_touches_the_record_but_the_listener_still_ticks() {
        let (settings, mut stable) = setup();
        stable.careers[0].strikes = 1;
        stable.careers[0].decay = 2;
        let result = PursuitResult { challenge_series: true, ..pursuit(false) };
        let end = end_player_pursuit(&settings, &mut stable, &result);
        assert_eq!(end.recorded, None);
        let r = &stable.careers[0];
        assert_eq!((r.bounty, r.evaded, r.unserved_total()), (0, 0, 0));
        assert_eq!((r.strikes, r.decay), (0, 0), "the evade tick ran before the recorder returned");
    }

    #[test]
    fn the_tick_runs_before_the_record_so_a_third_evade_removes_a_strike() {
        let (settings, mut stable) = setup();
        stable.careers[0].strikes = 2;
        for _ in 0..3 {
            end_player_pursuit(&settings, &mut stable, &pursuit(false));
        }
        assert_eq!(stable.careers[0].strikes, 1);
        assert_eq!(stable.careers[0].evaded, 3);
        assert_eq!(stable.careers[0].unserved[0], 3);
    }

    #[test]
    fn a_bust_does_not_decay_strikes() {
        let (settings, mut stable) = setup();
        stable.careers[0].strikes = 2;
        stable.careers[0].decay = 2;
        end_player_pursuit(&settings, &mut stable, &pursuit(true));
        assert_eq!((stable.careers[0].strikes, stable.careers[0].decay), (2, 2));
    }

    #[test]
    fn no_current_car_record_means_nothing_is_recorded() {
        let (mut settings, mut stable) = setup();
        settings.car_handle = 0xDEAD;
        let end = end_player_pursuit(&settings, &mut stable, &pursuit(false));
        assert_eq!(end.recorded, None);
        assert_eq!(stable.careers[0].evaded, 0);
    }

    #[test]
    fn end_pursuit_for_matches_end_player_pursuit() {
        for busted in [false, true] {
            let (settings, mut by_settings) = setup();
            let mut by_handle = by_settings.clone();
            let result = pursuit(busted);
            let a = end_player_pursuit(&settings, &mut by_settings, &result);
            let b = end_pursuit_for(settings.car_handle, &mut by_handle, &result);
            assert_eq!(a, b);
            assert_eq!(by_settings, by_handle);
        }
    }

    #[test]
    fn counters_and_bounty_saturate() {
        let mut r = CareerRecord::new(0);
        r.bounty = u32::MAX - 10;
        r.evaded = u16::MAX;
        let result = PursuitResult { rep_points_normal: u32::MAX, rep_points_destroyed: 1, ..pursuit(false) };
        assert_eq!(result.bounty(), u32::MAX);
        record_pursuit(&mut r, &result);
        assert_eq!((r.bounty, r.evaded), (u32::MAX, u16::MAX));
    }
}
