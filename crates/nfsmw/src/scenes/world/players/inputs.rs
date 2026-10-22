//! One frame of input per player (docs/specs/coop-career.md). Split-screen fills it from two local devices; a network
//! session later fills remote seats from received input. WorldSim::step(tick, &Inputs) keeps the same keying
//! (docs/specs/multiplayer/integration.md step 3a).
use super::PlayerId;
use crate::input::ActionState;

/// The actions of every seated player this frame, kept in player-id order.
#[derive(Default, Debug, Clone)]
pub struct Inputs {
    seats: Vec<(PlayerId, ActionState)>,
}

impl Inputs {
    /// Store `actions` for `id`, copying over that player's earlier entry in place.
    pub fn set(&mut self, id: PlayerId, actions: &ActionState) {
        match self.position(id) {
            Ok(index) => self.seats[index].1.clone_from(actions),
            Err(index) => self.seats.insert(index, (id, actions.clone())),
        }
    }

    /// The player's actions this frame, or `None` when that player has no seat.
    pub fn get(&self, id: PlayerId) -> Option<&ActionState> {
        let index = self.position(id).ok()?;
        Some(&self.seats[index].1)
    }

    /// Drop the player's seat, if it has one.
    #[cfg(test)]
    pub fn remove(&mut self, id: PlayerId) {
        let Ok(index) = self.position(id) else {
            return;
        };
        self.seats.remove(index);
    }

    /// Drop every seat.
    pub fn clear(&mut self) {
        self.seats.clear();
    }

    /// The seated players, in id order.
    #[cfg(test)]
    pub fn ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.seats.iter().map(|(id, _)| *id)
    }

    /// `Ok` is the seat's index. `Err` is where a new seat for `id` goes to keep the order.
    fn position(&self, id: PlayerId) -> Result<usize, usize> {
        self.seats.binary_search_by(|(seat, _)| seat.cmp(&id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Action, Bindings};
    use bevy_input::keyboard::KeyCode;

    /// The actions with `key` held, resolved the way the game does each frame.
    fn holding(key: KeyCode) -> ActionState {
        let mut actions = ActionState::default();
        actions.update_for_test(&Bindings::default(), &[key], &[], false);
        actions
    }

    #[test]
    fn set_then_get_by_player() {
        let mut inputs = Inputs::default();
        inputs.set(PlayerId(0), &holding(KeyCode::KeyW));
        inputs.set(PlayerId(1), &holding(KeyCode::KeyS));
        assert_eq!(inputs.get(PlayerId(0)).unwrap().value(Action::MoveForward), 1.0);
        assert_eq!(inputs.get(PlayerId(1)).unwrap().value(Action::MoveForward), -1.0);
    }

    #[test]
    fn unknown_player_is_none() {
        let mut inputs = Inputs::default();
        inputs.set(PlayerId(0), &holding(KeyCode::KeyW));
        assert!(inputs.get(PlayerId(1)).is_none());
        inputs.remove(PlayerId(0));
        assert!(inputs.get(PlayerId(0)).is_none());
    }

    #[test]
    fn set_replaces_in_place() {
        let mut inputs = Inputs::default();
        inputs.set(PlayerId(0), &holding(KeyCode::KeyW));
        inputs.set(PlayerId(1), &holding(KeyCode::KeyW));
        inputs.set(PlayerId(0), &holding(KeyCode::KeyS));
        assert_eq!(inputs.get(PlayerId(0)).unwrap().value(Action::MoveForward), -1.0);
        assert_eq!(inputs.get(PlayerId(1)).unwrap().value(Action::MoveForward), 1.0);
        assert_eq!(inputs.ids().collect::<Vec<_>>(), [PlayerId(0), PlayerId(1)]);
    }

    #[test]
    fn ids_stay_in_order() {
        let mut inputs = Inputs::default();
        inputs.set(PlayerId(2), &holding(KeyCode::KeyW));
        inputs.set(PlayerId(0), &holding(KeyCode::KeyW));
        inputs.set(PlayerId(1), &holding(KeyCode::KeyW));
        assert_eq!(inputs.ids().collect::<Vec<_>>(), [PlayerId(0), PlayerId(1), PlayerId(2)]);
        inputs.clear();
        assert_eq!(inputs.ids().count(), 0);
    }
}
