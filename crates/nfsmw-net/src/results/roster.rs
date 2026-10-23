//! Which slot and entity id each player has, so results and mode events can name players on the wire.

use nfsmw_sim::{EntityId, PlayerId};

use crate::message::is_slot;

/// One player's place in the session's id spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seat {
    pub player: PlayerId,
    /// The lobby slot, `0..32`.
    pub slot: u8,
    /// The player's car.
    pub entity: EntityId,
}

/// The seats of a session, in player-id order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roster {
    seats: Vec<Seat>,
}

impl Roster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces the seat of `seat.player`. Returns `false` (and changes nothing) for a slot outside the id space
    /// or a slot another player holds.
    pub fn insert(&mut self, seat: Seat) -> bool {
        if !is_slot(seat.slot) {
            return false;
        }
        if self.seats.iter().any(|other| other.slot == seat.slot && other.player != seat.player) {
            return false;
        }
        match self.seats.binary_search_by_key(&seat.player, |other| other.player) {
            Ok(index) => self.seats[index] = seat,
            Err(index) => self.seats.insert(index, seat),
        }
        true
    }

    /// Removes the seat of `player`.
    pub fn remove(&mut self, player: PlayerId) {
        self.seats.retain(|seat| seat.player != player);
    }

    /// The seat of `player`.
    pub fn get(&self, player: PlayerId) -> Option<Seat> {
        let index = self.seats.binary_search_by_key(&player, |seat| seat.player).ok()?;
        Some(self.seats[index])
    }

    /// The player whose car is `entity`.
    pub fn player_of(&self, entity: EntityId) -> Option<PlayerId> {
        self.seats.iter().find(|seat| seat.entity == entity).map(|seat| seat.player)
    }

    /// Every seat, in player-id order.
    pub fn seats(&self) -> &[Seat] {
        &self.seats
    }
}
