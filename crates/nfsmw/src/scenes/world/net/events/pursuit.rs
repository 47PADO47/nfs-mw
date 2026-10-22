//! Pursuit events per player from the chase's state after each tick (ADR 0010 section 4: one chase, several targets).
//!
//! A player is chased while the chase runs and lists it (`players/chase.rs`; outside co-op the police's one player).
//! A chased player that is busted, by its own meter mid-chase or by the police's bust at the end, gets `Busted` and
//! then `PursuitEnded`; one that the chase stops listing gets `PursuitEnded`, after `Evaded` when the whole chase ended
//! in an escape. A player busted in a chase does not start it again before that chase is over.

use nfsmw_sim::{PlayerId, SimEvent};

/// The chase as the world has it after a tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChaseState {
    /// A chase runs.
    pub pursuing: bool,
    /// The players the chase lists as chased.
    pub chased: Vec<PlayerId>,
    /// The players the chase lists as busted.
    pub busted: Vec<PlayerId>,
    /// The police's busted flag (`TrafficWorld::is_busted`): set when their meter fills on the player they follow, kept
    /// until the next chase.
    pub police_busted: bool,
    /// The player the police follow.
    pub primary: PlayerId,
    /// The last chase ended in an escape (`EndReason::Evaded`).
    pub evaded: bool,
}

/// What the last tick's chase was, per player.
#[derive(Debug, Clone, Default)]
pub struct PursuitTracker {
    chased: Vec<PlayerId>,
    /// Busted in the running chase: not started again until it is over.
    busted: Vec<PlayerId>,
    police_busted: bool,
}

impl PursuitTracker {
    /// Compare the chase after this tick with the last one; the events go to `events`, in player-id order per kind.
    pub fn update(&mut self, now: &ChaseState, events: &mut Vec<SimEvent>) {
        let police_bust = now.police_busted && !self.police_busted;
        self.police_busted = now.police_busted;
        let was_busted = |id: &PlayerId| {
            now.busted.contains(id) || (police_bust && *id == now.primary) || (!now.pursuing && police_bust)
        };
        let previous = std::mem::take(&mut self.chased);
        for player in previous.iter().copied() {
            if was_busted(&player) && !self.busted.contains(&player) {
                events.push(SimEvent::Busted { player });
                events.push(SimEvent::PursuitEnded { player });
                self.busted.push(player);
                continue;
            }
            if now.pursuing && now.chased.contains(&player) {
                continue;
            }
            if self.busted.contains(&player) {
                continue;
            }
            if !now.pursuing && now.evaded {
                events.push(SimEvent::Evaded { player });
            }
            events.push(SimEvent::PursuitEnded { player });
        }
        if !now.pursuing {
            self.busted.clear();
            return;
        }
        let mut chased: Vec<PlayerId> =
            now.chased.iter().copied().filter(|id| !self.busted.contains(id) && !now.busted.contains(id)).collect();
        chased.sort();
        chased.dedup();
        for player in chased.iter().copied().filter(|id| !previous.contains(id)) {
            events.push(SimEvent::PursuitStarted { player });
        }
        self.chased = chased;
    }

    /// The players chased after the last update, in id order.
    pub fn chased(&self) -> &[PlayerId] {
        &self.chased
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: PlayerId = PlayerId(0);
    const TWO: PlayerId = PlayerId(1);

    fn chase(chased: &[PlayerId]) -> ChaseState {
        ChaseState { pursuing: true, chased: chased.to_vec(), ..ChaseState::default() }
    }

    fn run(tracker: &mut PursuitTracker, state: ChaseState) -> Vec<SimEvent> {
        let mut events = Vec::new();
        tracker.update(&state, &mut events);
        events
    }

    #[test]
    fn a_chase_starts_and_ends_in_an_escape() {
        let mut tracker = PursuitTracker::default();
        assert_eq!(run(&mut tracker, ChaseState::default()), []);
        assert_eq!(run(&mut tracker, chase(&[ONE])), [SimEvent::PursuitStarted { player: ONE }]);
        assert_eq!(run(&mut tracker, chase(&[ONE])), [], "nothing while it runs");
        let over = ChaseState { evaded: true, ..ChaseState::default() };
        assert_eq!(run(&mut tracker, over), [SimEvent::Evaded { player: ONE }, SimEvent::PursuitEnded { player: ONE }]);
    }

    #[test]
    fn a_chase_stopped_without_a_result_only_ends() {
        let mut tracker = PursuitTracker::default();
        run(&mut tracker, chase(&[ONE]));
        assert_eq!(run(&mut tracker, ChaseState::default()), [SimEvent::PursuitEnded { player: ONE }]);
    }

    #[test]
    fn a_second_target_joins_and_is_busted_on_its_own_meter() {
        let mut tracker = PursuitTracker::default();
        run(&mut tracker, chase(&[ONE]));
        assert_eq!(run(&mut tracker, chase(&[ONE, TWO])), [SimEvent::PursuitStarted { player: TWO }]);
        let busted = ChaseState { busted: vec![TWO], ..chase(&[ONE]) };
        assert_eq!(
            run(&mut tracker, busted.clone()),
            [SimEvent::Busted { player: TWO }, SimEvent::PursuitEnded { player: TWO }]
        );
        assert_eq!(run(&mut tracker, busted), [], "a busted target does not start again in the same chase");
        assert_eq!(tracker.chased(), [ONE]);
    }

    #[test]
    fn the_police_bust_ends_the_chase_for_the_player_they_follow() {
        let mut tracker = PursuitTracker::default();
        run(&mut tracker, chase(&[TWO]));
        let bust = ChaseState { police_busted: true, primary: TWO, ..ChaseState::default() };
        assert_eq!(
            run(&mut tracker, bust.clone()),
            [SimEvent::Busted { player: TWO }, SimEvent::PursuitEnded { player: TWO }]
        );
        assert_eq!(run(&mut tracker, bust), [], "the flag stays set until the next chase");
        assert_eq!(run(&mut tracker, chase(&[TWO])), [SimEvent::PursuitStarted { player: TWO }], "a new chase");
    }

    #[test]
    fn a_player_that_leaves_the_chase_list_ends_its_pursuit() {
        let mut tracker = PursuitTracker::default();
        run(&mut tracker, chase(&[ONE, TWO]));
        assert_eq!(run(&mut tracker, chase(&[ONE])), [SimEvent::PursuitEnded { player: TWO }]);
    }
}
