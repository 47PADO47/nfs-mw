//! Co-op catch-up (`docs/specs/coop-career.md` section 10, ADR 0010). When the trailing player is past the soft tether from
//! the anchor and is not a pursuit target, they are offered "Press Reset to catch up". Their Reset press warps them behind
//! the anchor. Nothing forces the warp, and a declined prompt leaves the player where they are.

use glam::Vec3;

use super::super::WorldScene;
use super::coop::GUEST;
use super::tether::{self, Tether};
use super::{LEAD, PlayerId};
use crate::input::{Action, ActionState};

/// How far behind the anchor a caught-up player is placed, in metres.
const CATCH_UP_BEHIND: f32 = 12.0;

/// The two co-op players in id order: index 0 is the lead, index 1 the guest.
const SEATS: [PlayerId; 2] = [LEAD, GUEST];

/// The seat index of the player offered the catch-up prompt, or `None`. The anchor is the first busy seat
/// (`tether::anchor`), and the other seat trails. The trailing seat is offered the prompt when the tether prompts and it
/// is not a pursuit target.
fn prompted(anchor_busy: [bool; 2], positions: [Vec3; 2], targets: [bool; 2], tether: &Tether) -> Option<usize> {
    let anchor = tether::anchor(&anchor_busy);
    let trailing = 1 - anchor;
    let state = tether.judge(positions[anchor], positions[trailing]);
    match tether::may_catch_up(targets[trailing], state) {
        true => Some(trailing),
        false => None,
    }
}

impl WorldScene {
    /// Offer the catch-up prompt each frame and warp the trailing player on their Reset press. Runs after the simulation,
    /// so the warp replaces the respawn request that the ordinary reset just made. Outside co-op it does nothing.
    pub(in crate::scenes::world) fn update_catch_up(&mut self, lead_input: &ActionState) {
        let Some(session) = self.coop.as_ref() else { return };
        let tether = session.tether;
        let chased = self.traffic.as_ref().is_some_and(|traffic| traffic.is_pursuing());
        let racing = self.race.is_some();
        let (Some(lead), Some(guest)) = (self.players.lead(), self.players.get(GUEST)) else {
            self.set_catch_up(None);
            return;
        };
        let positions = [lead.position(), guest.position()];
        // The pursuit chases only the lead in this wave, so the guest is never a target.
        let anchor_busy = [chased || racing, racing];
        let offered = prompted(anchor_busy, positions, [chased, false], &tether).map(|seat| SEATS[seat]);
        self.set_catch_up(offered);
        let Some(id) = offered else { return };
        if !self.reset_pressed(id, lead_input) {
            return;
        }
        let anchor = SEATS[tether::anchor(&anchor_busy)];
        self.warp_behind(id, anchor);
        self.set_catch_up(None);
    }

    /// The prompt shown to `id` while they are offered a catch-up.
    pub(in crate::scenes::world) fn catch_up_message(&self, id: PlayerId) -> Option<&'static str> {
        match self.takes_reset_for_catch_up(id) {
            true => Some("Press Reset to catch up"),
            false => None,
        }
    }

    /// Whether `id` holds the catch-up prompt, so their Reset press is a catch-up and not an ordinary reset.
    pub(in crate::scenes::world) fn takes_reset_for_catch_up(&self, id: PlayerId) -> bool {
        self.coop.as_ref().is_some_and(|session| session.catch_up == Some(id))
    }

    /// Whether `id`'s Reset was pressed this frame. The lead's input comes from the frame; a guest's from `inputs`.
    fn reset_pressed(&self, id: PlayerId, lead_input: &ActionState) -> bool {
        let input = match id == LEAD {
            true => Some(lead_input),
            false => self.inputs.get(id),
        };
        input.is_some_and(|actions| actions.just_pressed(Action::ResetCar))
    }

    /// Put `id`'s car on the road `CATCH_UP_BEHIND` metres behind `anchor`'s car, facing the way the anchor faces.
    fn warp_behind(&mut self, id: PlayerId, anchor: PlayerId) {
        let Some(front) = self.players.get(anchor) else { return };
        let (position, heading) = (front.position(), front.heading());
        let forward = Vec3::new(heading.cos(), heading.sin(), 0.0);
        let at = tether::behind(position, forward, CATCH_UP_BEHIND);
        let Some(trailing) = self.players.get_mut(id) else { return };
        trailing.respawn_near([at.x, at.y], Some(heading));
        log::info!("co-op: player {} catches up behind player {}", id.0 + 1, anchor.0 + 1);
    }

    /// Set the player offered the prompt this frame. Does nothing outside co-op.
    fn set_catch_up(&mut self, offered: Option<PlayerId>) {
        let Some(session) = self.coop.as_mut() else { return };
        session.catch_up = offered;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAR: f32 = 600.0;

    /// The lead at the origin and the guest 300 m away: inside the soft tether.
    fn near() -> [Vec3; 2] {
        [Vec3::ZERO, Vec3::new(300.0, 0.0, 0.0)]
    }

    /// The lead at the origin and the guest `FAR` metres away: past the soft tether.
    fn far() -> [Vec3; 2] {
        [Vec3::ZERO, Vec3::new(FAR, 0.0, 0.0)]
    }

    #[test]
    fn no_prompt_when_near() {
        assert_eq!(prompted([false, false], near(), [false, false], &Tether::default()), None);
        assert_eq!(prompted([false, true], near(), [false, false], &Tether::default()), None);
    }

    #[test]
    fn trailing_player_is_prompted() {
        // Neither is busy, so the lead anchors and the guest trails.
        assert_eq!(prompted([false, false], far(), [false, false], &Tether::default()), Some(1));
    }

    #[test]
    fn no_catch_up_while_chased() {
        // The guest anchors and the lead trails while chased: no prompt for a chased car.
        assert_eq!(prompted([false, true], far(), [true, false], &Tether::default()), None);
        // The chased lead anchors, so the guest trails and is prompted.
        assert_eq!(prompted([true, false], far(), [true, false], &Tether::default()), Some(1));
    }

    #[test]
    fn busy_player_is_the_anchor() {
        // Only the guest is busy: the guest anchors and the lead trails.
        assert_eq!(prompted([false, true], far(), [false, false], &Tether::default()), Some(0));
        // Both busy: the first (the lead) anchors, so the guest trails.
        assert_eq!(prompted([true, true], far(), [false, false], &Tether::default()), Some(1));
    }
}
