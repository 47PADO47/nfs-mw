//! Contacts between the players' cars (`docs/specs/coop-career.md`): every pair of players pushes apart like any two
//! cars. Single player has no pair, so this does nothing there.

use super::super::drive::CarSim;

/// Lets the lead (when it stepped) and every guest hit each other, each pair once, the lower id first.
pub(in crate::scenes::world) fn collide_players(lead: Option<&mut CarSim>, guests: &mut [&mut CarSim]) {
    if let Some(lead) = lead {
        for guest in guests.iter_mut() {
            lead.collide_with_each(guest, |_, _, _| {});
        }
    }
    for i in 0..guests.len() {
        let (head, tail) = guests.split_at_mut(i + 1);
        for other in tail.iter_mut() {
            head[i].collide_with_each(other, |_, _, _| {});
        }
    }
}
