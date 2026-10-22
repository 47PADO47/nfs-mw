//! Lobby tests: shared fixtures.

mod admin;
mod rate;
mod roster;
mod start;

use super::*;
use crate::join::Placement;
use crate::loadout::{CarInfo, Catalog, Loadout};
use crate::message::{Name, ServerName, Settings};
use crate::roles::Rank;

/// Cars 0 to 9; cars 5 and up are class 2, the others class 1.
pub(super) struct Cars;

impl Catalog for Cars {
    fn car(&self, car_type: u32) -> Option<CarInfo> {
        (car_type < 10).then_some(CarInfo { class: if_class(car_type) })
    }
    fn part_fits(&self, _: u32, _: u16, _: u32) -> bool {
        true
    }
    fn paint_known(&self, _: u32) -> bool {
        true
    }
    fn wheel_set_known(&self, _: u32) -> bool {
        true
    }
    fn performance(&self, _: &Loadout) -> Option<u16> {
        Some(100)
    }
}

fn if_class(car_type: u32) -> u8 {
    1 + u8::from(car_type >= 5)
}

pub(super) fn lobby_with(settings: Settings, config: LobbyConfig) -> Lobby {
    Lobby::new(config, settings, false).unwrap()
}

pub(super) fn lobby() -> Lobby {
    lobby_with(Settings::lan(ServerName::new("Rockport").unwrap()), LobbyConfig::default())
}

pub(super) fn member(id: u8, rank: Rank, placement: Placement) -> NewMember {
    NewMember {
        player_id: [id; 16],
        name: Name::new(&format!("player {id}")).unwrap(),
        rank,
        placement,
        spectate_only: false,
        loadout: Loadout::stock(1),
    }
}

/// Adds a member and returns its slot.
pub(super) fn join(lobby: &mut Lobby, id: u8, rank: Rank, placement: Placement) -> u8 {
    lobby.add(member(id, rank, placement), 0).unwrap().0
}

/// A lobby with the host in slot 0 and `players` racers after it.
pub(super) fn with_players(players: u8) -> Lobby {
    let mut lobby = lobby();
    join(&mut lobby, 0, Rank::Host, Placement::Racer);
    for id in 1..=players {
        join(&mut lobby, id, Rank::Player, Placement::Racer);
    }
    lobby
}

/// Everyone gets ready, each at its own time so no bucket runs dry.
pub(super) fn all_ready(lobby: &mut Lobby, now_ms: u64) {
    for slot in lobby.racer_slots() {
        lobby.set_ready(slot, true, now_ms, 0).unwrap();
    }
}

pub(super) fn broadcasts(actions: &[Action]) -> Vec<&crate::message::Message> {
    actions
        .iter()
        .filter_map(|action| match action {
            Action::Broadcast(message) => Some(message),
            _ => None,
        })
        .collect()
}
