//! The player roster of one game: local device seats and remote peer seats, kept in id order.

use super::PeerId;

/// A player's number in a party, 0 first. Player 0 is the lead (the single player's id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PlayerId(pub u8);

/// Where a player's input comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    /// A device on this machine; the game maps the slot to its own device type.
    Local { slot: u8 },
    /// A peer of a network session.
    Remote { peer: PeerId },
}

/// The players of one game, kept in id order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    max: usize,
    seats: Vec<(PlayerId, Seat)>,
}

/// Largest party a `PlayerId` can number: ids run 0 to 255.
const MAX_PARTY: usize = 256;

impl Party {
    /// An empty party that holds at most `max` players. `max` is clamped to 256.
    pub fn new(max: usize) -> Self {
        Self { max: max.min(MAX_PARTY), seats: Vec::new() }
    }

    /// Seats a player in the lowest free id and returns that id. `None` when the party is full.
    pub fn join(&mut self, seat: Seat) -> Option<PlayerId> {
        if self.seats.len() >= self.max {
            return None;
        }
        let id = (0..self.max).map(|i| PlayerId(i as u8)).find(|id| self.seat(*id).is_none())?;
        let at = self.seats.partition_point(|(other, _)| *other < id);
        self.seats.insert(at, (id, seat));
        Some(id)
    }

    /// Removes a player and frees its id. `false` when the id is not in the party.
    pub fn leave(&mut self, id: PlayerId) -> bool {
        let at = match self.position(id) {
            Some(at) => at,
            None => return false,
        };
        self.seats.remove(at);
        true
    }

    /// The seat of one player, or `None` if it is not in the party.
    pub fn seat(&self, id: PlayerId) -> Option<Seat> {
        let at = self.position(id)?;
        Some(self.seats[at].1)
    }

    /// Every player id in the party, in id order.
    pub fn ids(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.seats.iter().map(|(id, _)| *id)
    }

    /// The number of players seated.
    pub fn len(&self) -> usize {
        self.seats.len()
    }

    /// True when no player is seated.
    pub fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }

    /// The most players this party holds.
    pub fn max(&self) -> usize {
        self.max
    }

    /// The local players with their device slot, in id order.
    pub fn local(&self) -> impl Iterator<Item = (PlayerId, u8)> + '_ {
        self.seats.iter().filter_map(|(id, seat)| match seat {
            Seat::Local { slot } => Some((*id, *slot)),
            Seat::Remote { .. } => None,
        })
    }

    /// Index of a player's entry in `seats`, or `None`.
    fn position(&self, id: PlayerId) -> Option<usize> {
        self.seats.binary_search_by_key(&id, |(other, _)| *other).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn party_fills_lowest_free_id() {
        let mut party = Party::new(8);
        assert_eq!(party.join(Seat::Local { slot: 0 }), Some(PlayerId(0)));
        assert_eq!(party.join(Seat::Local { slot: 1 }), Some(PlayerId(1)));
        assert_eq!(party.join(Seat::Local { slot: 2 }), Some(PlayerId(2)));
        assert!(party.leave(PlayerId(1)));
        assert_eq!(party.join(Seat::Local { slot: 3 }), Some(PlayerId(1)));
        let ids: Vec<PlayerId> = party.ids().collect();
        assert_eq!(ids, vec![PlayerId(0), PlayerId(1), PlayerId(2)]);
    }

    #[test]
    fn party_refuses_past_max() {
        let mut party = Party::new(2);
        assert_eq!(party.join(Seat::Local { slot: 0 }), Some(PlayerId(0)));
        assert_eq!(party.join(Seat::Local { slot: 1 }), Some(PlayerId(1)));
        assert_eq!(party.join(Seat::Local { slot: 2 }), None);
        assert_eq!(party.len(), 2);
        assert_eq!(party.max(), 2);
    }

    #[test]
    fn party_clamps_max_to_256() {
        let party = Party::new(10_000);
        assert_eq!(party.max(), 256);
        assert!(party.is_empty());
    }

    #[test]
    fn leave_frees_the_id() {
        let mut party = Party::new(4);
        assert_eq!(party.join(Seat::Local { slot: 0 }), Some(PlayerId(0)));
        assert!(party.leave(PlayerId(0)));
        assert_eq!(party.seat(PlayerId(0)), None);
        assert!(!party.leave(PlayerId(0)));
        assert!(!party.leave(PlayerId(9)));
        assert_eq!(party.join(Seat::Local { slot: 0 }), Some(PlayerId(0)));
    }

    #[test]
    fn local_and_remote_seats_coexist() {
        let mut party = Party::new(4);
        party.join(Seat::Local { slot: 0 });
        party.join(Seat::Remote { peer: PeerId(7) });
        party.join(Seat::Local { slot: 1 });
        assert_eq!(party.seat(PlayerId(1)), Some(Seat::Remote { peer: PeerId(7) }));
        let local: Vec<(PlayerId, u8)> = party.local().collect();
        assert_eq!(local, vec![(PlayerId(0), 0), (PlayerId(2), 1)]);
    }
}
