//! Contact events from the tick's car contacts (`players::TickContact`): one event per pair of entities, the lower id
//! first, at the highest closing speed the pair had in the tick (a car and its trailer are one entity), in id order.

use nfsmw_sim::{EntityId, SimEvent};

use crate::scenes::world::players::{Party, TickContact};

/// The events of `contacts`, with each party's entity id from `id` (a party with none is left out: a car that left the
/// world in the same tick). Appended to `events`.
pub fn contact_events(contacts: &[TickContact], id: impl Fn(Party) -> Option<EntityId>, events: &mut Vec<SimEvent>) {
    let mut pairs: Vec<(EntityId, EntityId, f32)> = Vec::new();
    for contact in contacts {
        let (Some(a), Some(b)) = (id(contact.a), id(contact.b)) else { continue };
        if a == b {
            continue;
        }
        let (a, b) = (a.min(b), a.max(b));
        let speed = match contact.speed.is_finite() {
            true => contact.speed,
            false => 0.0,
        };
        match pairs.iter_mut().find(|pair| (pair.0, pair.1) == (a, b)) {
            Some(pair) => pair.2 = pair.2.max(speed),
            None => pairs.push((a, b, speed)),
        }
    }
    pairs.sort_by_key(|&(a, b, _)| (a, b));
    events.extend(pairs.into_iter().map(|(a, b, speed)| SimEvent::Contact { a, b, speed }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenes::world::players::PlayerId;

    fn id(party: Party) -> Option<EntityId> {
        match party {
            Party::Player(player) => Some(EntityId::from(player.0)),
            Party::Car(7) => None,
            Party::Car(car) => Some(100 + car as EntityId),
        }
    }

    #[test]
    fn each_pair_once_lower_id_first_fastest_speed_in_id_order() {
        let contact = |a, b, speed| TickContact { a, b, speed };
        let contacts = [
            contact(Party::Car(2), Party::Player(PlayerId(1)), 3.0),
            contact(Party::Player(PlayerId(0)), Party::Player(PlayerId(1)), 5.0),
            contact(Party::Player(PlayerId(1)), Party::Car(2), 9.0),
            contact(Party::Player(PlayerId(0)), Party::Car(7), 1.0),
            contact(Party::Car(2), Party::Car(2), 1.0),
            contact(Party::Player(PlayerId(3)), Party::Car(4), f32::NAN),
        ];
        let mut events = Vec::new();
        contact_events(&contacts, id, &mut events);
        assert_eq!(
            events,
            [
                SimEvent::Contact { a: 0, b: 1, speed: 5.0 },
                SimEvent::Contact { a: 1, b: 102, speed: 9.0 },
                SimEvent::Contact { a: 3, b: 104, speed: 0.0 },
            ]
        );
    }
}
