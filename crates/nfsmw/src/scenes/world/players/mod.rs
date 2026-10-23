//! The players of the world (ADR 0010, `docs/specs/coop-career.md`). Player 1 (`PlayerId(0)`) is the lead: single
//! player is a list of one, and every system that knows one car reads the lead. Co-op adds player 2 with a full
//! [`Drive`] of its own, the same path a remote player of a network session takes later.

mod catch_up;
pub mod chase;
pub(super) mod command;
mod contact;
pub mod coop;
pub mod inputs;
mod race;
pub mod spawn;
mod step;
mod suspend;
pub mod tether;
mod view;

pub use blackbox_net_core::session::PlayerId;
pub use contact::{Party, TickContact};
pub(super) use contact::{collide_players, touch_contact};

use super::drive::Drive;

/// The lead player's id.
pub const LEAD: PlayerId = PlayerId(0);

/// One player's car in the world.
pub struct Player<D = Drive> {
    pub id: PlayerId,
    pub drive: D,
    /// Out of play for now: the car neither steps, collides nor shows, and no system counts the player (a co-op guest
    /// during a race, where they drive a race seat, or during a cutscene).
    pub parked: bool,
}

impl<D> Player<D> {
    pub fn new(id: PlayerId, drive: D) -> Self {
        Self { id, drive, parked: false }
    }
}

/// The players in id order; the lead is first when present. Generic over the car so the seat rules are tested
/// without a world.
pub struct Players<D = Drive> {
    list: Vec<Player<D>>,
    /// The contacts of the players' cars in the last tick, with each other and with the traffic (`contact.rs`).
    pub contacts: Vec<TickContact>,
}

impl<D> Default for Players<D> {
    fn default() -> Self {
        Self { list: Vec::new(), contacts: Vec::new() }
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
        self.insert(Player::new(LEAD, drive));
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

    /// The players after the lead (co-op guests) in play, in id order.
    pub fn guests(&self) -> impl Iterator<Item = &Player<D>> {
        self.list.iter().filter(|p| p.id != LEAD && !p.parked)
    }

    pub fn guests_mut(&mut self) -> impl Iterator<Item = &mut Player<D>> {
        self.list.iter_mut().filter(|p| p.id != LEAD && !p.parked)
    }

    /// Takes player `id` out of play (`true`) or puts it back. Returns whether there is such a player.
    pub fn park(&mut self, id: PlayerId, parked: bool) -> bool {
        let Some(player) = self.list.iter_mut().find(|p| p.id == id) else { return false };
        player.parked = parked;
        true
    }

    /// Whether player `id` is in the world but out of play.
    pub fn is_parked(&self, id: PlayerId) -> bool {
        self.list.iter().any(|p| p.id == id && p.parked)
    }

    /// The first guest's car: the second view of a split screen.
    pub fn first_guest(&self) -> Option<&D> {
        self.guests().next().map(|p| &p.drive)
    }

    pub fn has_guests(&self) -> bool {
        self.guests().next().is_some()
    }

    /// The lead's car and the guests in play at once, for steps that touch both.
    pub fn lead_and_guests_mut(&mut self) -> (Option<&mut D>, Vec<&mut Player<D>>) {
        let lead_count = usize::from(self.list.first().is_some_and(|p| p.id == LEAD));
        let (lead, rest) = self.list.split_at_mut(lead_count);
        (lead.first_mut().map(|p| &mut p.drive), rest.iter_mut().filter(|p| !p.parked).collect())
    }

    /// Every player's car, the lead first.
    #[cfg(test)]
    pub fn drives(&self) -> impl Iterator<Item = &D> {
        self.list.iter().map(|p| &p.drive)
    }

    /// Every car in play, the lead first.
    pub fn drives_mut(&mut self) -> impl Iterator<Item = &mut D> {
        self.list.iter_mut().filter(|p| !p.parked).map(|p| &mut p.drive)
    }

    /// Every player's id in id order, the parked ones too.
    pub fn ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.list.iter().map(|p| p.id)
    }

    /// Every player in play, the lead first.
    pub fn iter(&self) -> impl Iterator<Item = &Player<D>> {
        self.list.iter().filter(|p| !p.parked)
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
        players.insert(Player::new(PlayerId(1), "two"));
        assert_eq!(players.lead(), None);
        players.set_lead("one");
        assert_eq!(players.lead(), Some(&"one"));
        assert_eq!(players.drives().copied().collect::<Vec<_>>(), ["one", "two"]);
    }

    #[test]
    fn guests_are_everyone_in_play_but_the_lead() {
        let mut players = Players::default();
        players.insert(Player::new(PlayerId(2), 3));
        players.set_lead(1);
        players.insert(Player::new(PlayerId(1), 2));
        assert_eq!(players.guests().map(|p| p.drive).collect::<Vec<_>>(), [2, 3]);
        let (lead, guests) = players.lead_and_guests_mut();
        assert_eq!(lead.copied(), Some(1));
        assert_eq!(guests.len(), 2);
        assert!(players.park(PlayerId(1), true));
        assert!(players.is_parked(PlayerId(1)));
        assert_eq!(players.guests().map(|p| p.drive).collect::<Vec<_>>(), [3], "a parked guest is out of play");
        assert_eq!(players.lead_and_guests_mut().1.len(), 1);
        assert_eq!(players.get(PlayerId(1)), Some(&2), "a parked guest keeps its car");
    }

    #[test]
    fn mutable_guests_work_without_a_lead() {
        let mut players = Players::<u8>::default();
        let (lead, guests) = players.lead_and_guests_mut();
        assert!(lead.is_none() && guests.is_empty());
        players.insert(Player::new(PlayerId(1), 2));
        let (lead, mut guests) = players.lead_and_guests_mut();
        assert!(lead.is_none());
        guests[0].drive = 3;
        assert_eq!(players.get(PlayerId(1)), Some(&3));
        players.park(PlayerId(1), true);
        assert!(players.lead_and_guests_mut().1.is_empty());
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
        players.insert(Player::new(PlayerId(1), 2));
        assert_eq!(players.remove(PlayerId(1)), Some(2));
        assert_eq!(players.take_lead(), Some(1));
        assert!(players.is_empty());
    }
}
