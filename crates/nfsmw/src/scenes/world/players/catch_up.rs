//! Co-op catch-up (`docs/specs/coop-career.md` section 10, ADR 0010). When a trailing player is past the soft tether from
//! the anchor and is not a pursuit target, they are offered "Press Reset to catch up". Their Reset press warps them behind
//! the anchor. Nothing forces the warp, and a declined prompt leaves the player where they are. Any number of players:
//! the anchor is the first busy one, and every other player in play is judged against it.

use glam::Vec3;

use super::super::WorldScene;
use super::tether::{self, Tether};
use super::{LEAD, PlayerId};
use crate::input::{Action, ActionState};

/// How far behind the anchor a caught-up player is placed, in metres.
const CATCH_UP_BEHIND: f32 = 12.0;

/// The seat indices offered the catch-up prompt, in seat order. The anchor is the first busy seat (`tether::anchor`), and
/// every other seat trails. A trailing seat is offered the prompt when the tether prompts between it and the anchor and
/// it is not a pursuit target.
fn prompted(busy: &[bool], positions: &[Vec3], targets: &[bool], tether: &Tether) -> Vec<usize> {
    let anchor = tether::anchor(busy);
    let Some(&at) = positions.get(anchor) else { return Vec::new() };
    let target = |seat: usize| targets.get(seat).copied().unwrap_or(false);
    (0..positions.len())
        .filter(|&seat| seat != anchor && tether::may_catch_up(target(seat), tether.judge(at, positions[seat])))
        .collect()
}

impl WorldScene {
    /// Offer the catch-up prompt each frame and warp a trailing player on their Reset press. Runs after the simulation,
    /// so the warp replaces the respawn request that the ordinary reset just made. Outside co-op it does nothing.
    pub(in crate::scenes::world) fn update_catch_up(&mut self, lead_input: &ActionState) {
        let Some(session) = self.coop.as_ref() else { return };
        let tether = session.tether;
        let pursuing = self.traffic.as_ref().is_some_and(|traffic| traffic.is_pursuing());
        let racing = self.race.is_some();
        let chase = &session.chase;
        // A chased player is a target while the chase runs; the police's own player is one too before the co-op chase
        // has listed it. The lead is busy in a race, and any target is busy.
        let target = |id: PlayerId| pursuing && (chase.is_chased(id) || chase.primary == id);
        let seats: Vec<(PlayerId, Vec3)> = self.players.iter().map(|p| (p.id, p.drive.position())).collect();
        let busy: Vec<bool> = seats.iter().map(|&(id, _)| target(id) || (racing && id == LEAD)).collect();
        let targets: Vec<bool> = seats.iter().map(|&(id, _)| target(id)).collect();
        let positions: Vec<Vec3> = seats.iter().map(|&(_, at)| at).collect();
        let offered: Vec<PlayerId> =
            prompted(&busy, &positions, &targets, &tether).into_iter().map(|seat| seats[seat].0).collect();
        let anchor = seats.get(tether::anchor(&busy)).map(|&(id, _)| id);
        let pressed: Vec<PlayerId> = offered.iter().copied().filter(|&id| self.reset_pressed(id, lead_input)).collect();
        self.set_catch_up(offered);
        let Some(anchor) = anchor else { return };
        for id in pressed {
            self.warp_behind(id, anchor);
            if let Some(session) = self.coop.as_mut() {
                session.catch_up.retain(|&offered| offered != id);
            }
        }
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
        self.coop.as_ref().is_some_and(|session| session.catch_up.contains(&id))
    }

    /// Whether `id`'s Reset was pressed this frame. The lead's input comes from the frame; a local guest's from
    /// `inputs`; a network seat's from the input the server gave its car.
    fn reset_pressed(&self, id: PlayerId, lead_input: &ActionState) -> bool {
        if self.players.get(id).is_some_and(|drive| drive.remote_reset()) {
            return true;
        }
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

    /// Set the players offered the prompt this frame. Does nothing outside co-op.
    fn set_catch_up(&mut self, offered: Vec<PlayerId>) {
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

    fn none() -> Vec<usize> {
        Vec::new()
    }

    #[test]
    fn no_prompt_when_near() {
        assert_eq!(prompted(&[false, false], &near(), &[false, false], &Tether::default()), none());
        assert_eq!(prompted(&[false, true], &near(), &[false, false], &Tether::default()), none());
    }

    #[test]
    fn trailing_player_is_prompted() {
        // Neither is busy, so the lead anchors and the guest trails.
        assert_eq!(prompted(&[false, false], &far(), &[false, false], &Tether::default()), [1]);
    }

    #[test]
    fn no_catch_up_while_chased() {
        // The guest anchors and the lead trails while chased: no prompt for a chased car.
        assert_eq!(prompted(&[false, true], &far(), &[true, false], &Tether::default()), none());
        // The chased lead anchors, so the guest trails and is prompted.
        assert_eq!(prompted(&[true, false], &far(), &[true, false], &Tether::default()), [1]);
    }

    #[test]
    fn busy_player_is_the_anchor() {
        // Only the guest is busy: the guest anchors and the lead trails.
        assert_eq!(prompted(&[false, true], &far(), &[false, false], &Tether::default()), [0]);
        // Both busy: the first (the lead) anchors, so the guest trails.
        assert_eq!(prompted(&[true, true], &far(), &[false, false], &Tether::default()), [1]);
    }

    #[test]
    fn every_trailing_player_of_many_is_judged_against_the_anchor() {
        let positions = [Vec3::ZERO, Vec3::new(FAR, 0.0, 0.0), Vec3::new(50.0, 0.0, 0.0), Vec3::new(0.0, -FAR, 0.0)];
        let quiet = [false; 4];
        assert_eq!(prompted(&quiet, &positions, &quiet, &Tether::default()), [1, 3]);
        // Player 4 is a target: never prompted. Player 3 is busy and anchors, so the lead trails it by only 50 m.
        let busy = [false, false, true, false];
        let targets = [false, false, false, true];
        assert_eq!(prompted(&busy, &positions, &targets, &Tether::default()), [1]);
    }

    #[test]
    fn one_player_is_never_prompted() {
        assert_eq!(prompted(&[false], &[Vec3::ZERO], &[false], &Tether::default()), none());
        assert_eq!(prompted(&[], &[], &[], &Tether::default()), none());
    }
}
