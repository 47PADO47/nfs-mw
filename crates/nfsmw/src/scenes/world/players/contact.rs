//! Contacts between the players' cars (`docs/specs/coop-career.md`): every pair of players pushes apart like any two
//! cars. Single player has no pair, so this does nothing there. The contacts of the tick, with the players' contacts
//! with the traffic, are kept for a server's contact events (`docs/specs/multiplayer/world-host.md`).

use super::super::ai::{CarTouch, Toucher};
use super::super::drive::CarSim;
use super::PlayerId;

/// One side of a contact between cars in a tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Party {
    Player(PlayerId),
    /// A traffic, police or roadblock car, by its entity id in the traffic world (`AiCar::key`).
    Car(u32),
}

/// Two cars that touched in the last tick and how fast they closed before (m/s).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickContact {
    pub a: Party,
    pub b: Party,
    pub speed: f32,
}

/// Lets the lead (when it stepped) and every guest hit each other, each pair once, the lower id first. `guest_ids` names
/// the cars of `guests` (and `lead` is the car the police follow, with its id); each pair that touched goes to
/// `contacts`.
pub(in crate::scenes::world) fn collide_players(
    lead: Option<(PlayerId, &mut CarSim)>,
    guests: &mut [&mut CarSim],
    guest_ids: &[PlayerId],
    contacts: &mut Vec<TickContact>,
) {
    let id = |index: usize| guest_ids.get(index).copied().unwrap_or(PlayerId(u8::MAX));
    if let Some((lead_id, lead)) = lead {
        for (index, guest) in guests.iter_mut().enumerate() {
            let speed = closing(lead, guest);
            if lead.collide_with_each(guest, |_, _, _| {}) {
                contacts.push(TickContact { a: Party::Player(lead_id), b: Party::Player(id(index)), speed });
            }
        }
    }
    for i in 0..guests.len() {
        let (head, tail) = guests.split_at_mut(i + 1);
        for (offset, other) in tail.iter_mut().enumerate() {
            let speed = closing(head[i], other);
            if head[i].collide_with_each(other, |_, _, _| {}) {
                contacts.push(TickContact { a: Party::Player(id(i)), b: Party::Player(id(i + 1 + offset)), speed });
            }
        }
    }
}

/// A traffic contact as players and cars: `followed` is the car the traffic's collision was given (`None` when it was
/// given none), `guests` the ids of the cars its guest collision was given, in order.
pub(in crate::scenes::world) fn touch_contact(
    touch: &CarTouch,
    followed: Option<PlayerId>,
    guests: &[PlayerId],
) -> Option<TickContact> {
    let party = |side: Toucher| match side {
        Toucher::Followed => followed.map(Party::Player),
        Toucher::Guest(index) => guests.get(index).copied().map(Party::Player),
        Toucher::Car(entity) => Some(Party::Car(entity)),
    };
    Some(TickContact { a: party(touch.a)?, b: party(touch.b)?, speed: touch.speed })
}

/// How fast two cars close (m/s).
fn closing(a: &CarSim, b: &CarSim) -> f32 {
    (a.body().linear_velocity - b.body().linear_velocity).length()
}
