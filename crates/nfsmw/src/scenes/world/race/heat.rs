//! A custom race's Heat Level: the police the race starts with (`docs/specs/custom-race.md` section 4). Above None a chase
//! starts at the green light at the level's heat and runs the race through; a bust ends the race as a loss. The chase is
//! the one the world runs (`ai::pursuit_command`), as a pursuit challenge starts it. **[ours]**

use blackbox_pursuit::EndReason;
use blackbox_race::Phase;

use super::finish::RACE_OVER_SECS;
use super::{Banner, RaceResult, RaceSession, Stage};
use crate::scenes::world::WorldScene;

/// Where a race's chase stands: waiting for the green light, or begun (a bust then ends the race).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::scenes::world) enum HeatChase {
    Waiting(u32),
    Begun,
}

/// What a race's Heat Level asks of the police when the race starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Police {
    /// None: any chase still on is called off.
    CallOff,
    /// A pursuit race keeps the police its data gives it.
    Keep,
    /// A chase starts at the green light at this heat, and the car carries it into a later chase.
    Chase(u32),
}

impl Police {
    /// The heat of the chase this asks for, if any.
    pub(super) fn chase(self) -> Option<u32> {
        match self {
            Police::Chase(heat) => Some(heat),
            Police::CallOff | Police::Keep => None,
        }
    }
}

/// The pursuit heat of a Heat Level: Minimum 1, Moderate 3, Maximum 5 (`0` is None, which has no chase). **[ours]**
pub(super) fn chase_heat(level: Option<u8>) -> Option<u32> {
    match level? {
        1 => Some(1),
        2 => Some(3),
        3 => Some(5),
        _ => None,
    }
}

/// A chosen level overrides a track's ambient cops. Pursuit events still keep their required police.
pub(super) fn police_of(level: Option<u8>, pursuit_race: bool, cops_in_race: bool) -> Police {
    if let Some(heat) = chase_heat(level) {
        return Police::Chase(heat);
    }
    match pursuit_race || (level.is_none() && cops_in_race) {
        true => Police::Keep,
        false => Police::CallOff,
    }
}

impl WorldScene {
    /// One frame of a race with a chase. The chase starts once the GO is given and the car stands on the road; a bust
    /// after that ends the race as the player's last place, with no time (a loss, as a pursuit challenge's bust is).
    pub(super) fn heat_chase_frame(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        session: &mut RaceSession,
    ) -> bool {
        let Some(chase) = session.heat_chase else { return true };
        match chase {
            HeatChase::Waiting(heat) => self.begin_heat_chase(renderer, session, heat),
            HeatChase::Begun => self.heat_bust_check(session),
        }
        true
    }

    fn begin_heat_chase(
        &mut self,
        renderer: &mut dyn blackbox_gfx::RenderBackend,
        session: &mut RaceSession,
        heat: u32,
    ) {
        let green = session.engine.as_ref().is_some_and(|e| e.phase() == Phase::Racing);
        let standing = self.players.lead().is_some_and(|d| d.has_sim() && d.waiting_for_road().is_none());
        if !green || !standing {
            return;
        }
        session.heat_chase = Some(HeatChase::Begun);
        match crate::scenes::world::ai::start_event_pursuit(self, renderer, heat) {
            Ok(line) => log::info!("race {}: {line}", session.name),
            Err(why) => log::warn!("race {}: the chase did not start: {why}", session.name),
        }
    }

    fn heat_bust_check(&mut self, session: &mut RaceSession) {
        let busted = self.traffic.as_ref().is_some_and(|t| t.end_reason() == EndReason::Busted);
        if !busted {
            return;
        }
        // The bust has ended the chase and holds the car for the arrest, so the cops are not called off here.
        let racers = session.engine.as_ref().map_or(1, |e| e.racers().len());
        session.result = Some(RaceResult { place: racers as u32, racers, time: None });
        session.stage = Stage::Results { left: RACE_OVER_SECS };
        self.note_race_outcome(session);
        session.banner = Banner::default();
        log::info!("race: {} lost to a bust", session.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_heat_level_starts_its_own_chase() {
        assert_eq!(chase_heat(Some(1)), Some(1), "Minimum");
        assert_eq!(chase_heat(Some(2)), Some(3), "Moderate");
        assert_eq!(chase_heat(Some(3)), Some(5), "Maximum");
    }

    #[test]
    fn none_and_no_level_start_no_chase() {
        assert_eq!(chase_heat(Some(0)), None);
        assert_eq!(chase_heat(None), None);
        assert_eq!(chase_heat(Some(9)), None, "a level the screens do not offer");
    }

    #[test]
    fn only_none_calls_off_a_chase() {
        assert_eq!(police_of(Some(0), false, false), Police::CallOff);
        assert_eq!(police_of(None, false, false), Police::CallOff);
        assert_eq!(police_of(Some(1), false, false), Police::Chase(1));
        assert_eq!(police_of(Some(3), false, false), Police::Chase(5));
    }

    #[test]
    fn a_pursuit_race_keeps_its_police_when_none_is_chosen() {
        assert_eq!(police_of(None, true, false), Police::Keep);
        assert_eq!(police_of(Some(0), true, false), Police::Keep);
        assert_eq!(police_of(Some(2), true, false), Police::Chase(3), "a chosen level still starts its chase");
    }

    #[test]
    fn explicit_none_overrides_a_tracks_ambient_police_flag() {
        assert_eq!(police_of(None, false, true), Police::Keep, "career keeps the authored setting");
        assert_eq!(police_of(Some(0), false, true), Police::CallOff, "custom race disables police");
        assert_eq!(police_of(Some(1), false, true), Police::Chase(1));
    }

    #[test]
    fn the_chase_heat_is_the_carried_heat() {
        assert_eq!(Police::Chase(3).chase(), Some(3));
        assert_eq!(Police::CallOff.chase(), None);
        assert_eq!(Police::Keep.chase(), None);
    }
}
