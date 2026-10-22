//! The players of the world (ADR 0010, `docs/specs/coop-career.md`). Player 1 (`PlayerId(0)`) is the lead: single
//! player is a list of one, and every system that knows one car reads the lead. Co-op adds player 2 with a full
//! [`Drive`] of its own, the same path a remote player of a network session takes later.

mod catch_up;
pub(super) mod command;
mod contact;
pub mod coop;
pub mod inputs;
pub mod spawn;
mod step;
pub mod tether;
mod view;

pub use blackbox_net_core::session::PlayerId;
pub(super) use contact::collide_players;

use super::drive::Drive;

/// The lead player's id.
pub const LEAD: PlayerId = PlayerId(0);

/// One player's car in the world.
pub struct Player<D = Drive> {
    pub id: PlayerId,
    pub drive: D,
}

/// The players in id order; the lead is first when present. Generic over the car so the seat rules are tested
/// without a world.
pub struct Players<D = Drive> {
    list: Vec<Player<D>>,
}

impl<D> Default for Players<D> {
    fn default() -> Self {
        Self { list: Vec::new() }
    }
}

impl<D> Players<D> {
    /// The lead player's car.
    pub fn lead(&self) -> Option<&D> {
        self.get(LEAD)
    }

    pub fn lead_mut(&mut self) -> Option<&mut D> {
        self.get_mut(LEAD)
    }

    pub fn has_lead(&self) -> bool {
        self.lead().is_some()
    }

    /// Put `drive` in the lead's seat, replacing the car there.
    pub fn set_lead(&mut self, drive: D) {
        self.insert(Player { id: LEAD, drive });
    }

    /// Take the lead's car out of the world.
    #[cfg(test)]
    pub fn take_lead(&mut self) -> Option<D> {
        self.remove(LEAD)
    }

    pub fn get(&self, id: PlayerId) -> Option<&D> {
        self.list.iter().find(|p| p.id == id).map(|p| &p.drive)
    }

    pub fn get_mut(&mut self, id: PlayerId) -> Option<&mut D> {
        self.list.iter_mut().find(|p| p.id == id).map(|p| &mut p.drive)
    }

    /// Add `player`, replacing a player with the same id; keeps id order.
    pub fn insert(&mut self, player: Player<D>) {
        let at = self.list.partition_point(|p| p.id < player.id);
        if self.list.get(at).is_some_and(|p| p.id == player.id) {
            self.list[at] = player;
            return;
        }
        self.list.insert(at, player);
    }

    pub fn remove(&mut self, id: PlayerId) -> Option<D> {
        let at = self.list.iter().position(|p| p.id == id)?;
        Some(self.list.remove(at).drive)
    }

    /// The players after the lead (co-op guests), in id order.
    pub fn guests(&self) -> impl Iterator<Item = &Player<D>> {
        self.list.iter().filter(|p| p.id != LEAD)
    }

    pub fn guests_mut(&mut self) -> impl Iterator<Item = &mut Player<D>> {
        self.list.iter_mut().filter(|p| p.id != LEAD)
    }

    /// The first guest's car: the second view of a split screen.
    pub fn first_guest(&self) -> Option<&D> {
        self.guests().next().map(|p| &p.drive)
    }

    pub fn has_guests(&self) -> bool {
        self.guests().next().is_some()
    }

    /// The lead's car and the guests at once, for steps that touch both.
    pub fn lead_and_guests_mut(&mut self) -> (Option<&mut D>, &mut [Player<D>]) {
        match self.list.split_first_mut() {
            Some((first, rest)) if first.id == LEAD => (Some(&mut first.drive), rest),
            _ => (None, &mut self.list[..]),
        }
    }

    /// Every player's car, the lead first.
    #[cfg(test)]
    pub fn drives(&self) -> impl Iterator<Item = &D> {
        self.list.iter().map(|p| &p.drive)
    }

    pub fn drives_mut(&mut self) -> impl Iterator<Item = &mut D> {
        self.list.iter_mut().map(|p| &mut p.drive)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Player<D>> {
        self.list.iter()
    }

    /// Take every player out of the world, the lead first.
    pub fn take_all(&mut self) -> Vec<Player<D>> {
        std::mem::take(&mut self.list)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lead_is_player_zero() {
        let mut players = Players::default();
        players.insert(Player { id: PlayerId(1), drive: "two" });
        assert_eq!(players.lead(), None);
        players.set_lead("one");
        assert_eq!(players.lead(), Some(&"one"));
        assert_eq!(players.drives().copied().collect::<Vec<_>>(), ["one", "two"]);
    }

    #[test]
    fn insert_replaces_the_same_id() {
        let mut players = Players::default();
        players.set_lead(1);
        players.set_lead(2);
        assert_eq!(players.len(), 1);
        assert_eq!(players.lead(), Some(&2));
    }

    #[test]
    fn remove_and_take_all_empty_the_list() {
        let mut players = Players::default();
        players.set_lead(1);
        players.insert(Player { id: PlayerId(1), drive: 2 });
        assert_eq!(players.remove(PlayerId(1)), Some(2));
        assert_eq!(players.take_lead(), Some(1));
        assert!(players.is_empty());
    }
}
