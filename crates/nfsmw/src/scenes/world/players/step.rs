//! The frame work of the co-op guests (every player after the lead): their input, their Speedbreaker, their look and
//! putting a waiting car on the road. The lead keeps the single-player path in `simulation.rs` and `drive_input.rs`;
//! the physics tick of every player is in `simulation.rs` (`docs/specs/coop-career.md`).

use super::super::{WorldScene, ground, road};
use crate::input::ActionState;
use crate::settings::SpeedbreakerTuning;

impl WorldScene {
    /// Each guest samples its own input for this frame.
    pub(in crate::scenes::world) fn prepare_guests(&mut self, dt: f32, driving: bool, tuning: SpeedbreakerTuning) {
        let idle = ActionState::default();
        for guest in self.players.guests_mut() {
            guest.drive.set_speedbreaker_tuning(tuning);
            if driving {
                guest.drive.prepare_frame(self.inputs.get(guest.id).unwrap_or(&idle), dt);
            }
        }
    }

    /// The guests' Speedbreaker edges. The world has one clock, so a guest holding Speedbreaker slows it for everyone,
    /// as the lead's does [ours]. Every guest is asked, so each meter sees its own button.
    pub(in crate::scenes::world) fn guests_prepare_time(&mut self) -> bool {
        self.players.guests_mut().fold(false, |any, guest| guest.drive.prepare_time() | any)
    }

    pub(in crate::scenes::world) fn guests_finish_time(&mut self, sim_dt: f32, real_dt: f32, tick_sim_dt: f32) {
        let idle = ActionState::default();
        for guest in self.players.guests_mut() {
            guest.drive.finish_time_with_tick_delta(
                self.inputs.get(guest.id).unwrap_or(&idle),
                sim_dt,
                real_dt,
                tick_sim_dt,
            );
        }
    }

    /// The camera look each guest asked for over this frame's ticks, in guest order.
    pub(in crate::scenes::world) fn guests_take_look(&mut self, steps: u32) -> Vec<(f32, f32)> {
        self.players.guests_mut().map(|guest| guest.drive.take_look(steps)).collect()
    }

    /// The effects of a guest that is not on the road yet still fade.
    pub(in crate::scenes::world) fn age_guest_effects(&mut self, driving: bool, sim_dt: f32) {
        for guest in self.players.guests_mut() {
            if !driving || guest.drive.waiting_for_road().is_some() {
                guest.drive.age_effects(sim_dt);
            }
        }
    }

    pub(in crate::scenes::world) fn set_guest_alpha(&mut self, alpha: f32) {
        for guest in self.players.guests_mut() {
            guest.drive.set_alpha(alpha);
        }
    }

    /// Put a waiting guest on the road once the world around every player has loaded: the road nearest its request,
    /// else where it last stood on one. With neither, it joins the lead's road.
    pub(in crate::scenes::world) fn place_guests(&mut self) {
        if !self.residency.complete() {
            return;
        }
        let lead = self.players.lead().map(|drive| drive.position());
        let collision = self.residency.collision();
        let surfaces = &self.physics.surfaces;
        for guest in self.players.guests_mut() {
            let drive = &mut guest.drive;
            drive.set_markers(self.markers_on);
            let Some(request) = drive.waiting_for_road() else { continue };
            // A race start waits for its own ground and is never moved to another road.
            if request.strict
                && let Some(spawn) = request.exact
            {
                drive.place_strict(spawn, collision, surfaces);
                continue;
            }
            let estimate = ground::height_near(self.residency.placed(), request.near[0], request.near[1], 150.0);
            let top = estimate.unwrap_or(0.0) + 40.0;
            let placed = match request.exact {
                Some(spawn) => drive.spawn(spawn, collision, surfaces),
                None => road::find(request.near, |x, y| road::probe(collision, x, y, top))
                    .is_some_and(|spawn| drive.spawn(spawn, collision, surfaces)),
            };
            if placed {
                continue;
            }
            if drive.last_good().is_some_and(|good| drive.spawn(good, collision, surfaces)) {
                continue;
            }
            let Some(at) = lead else { continue };
            log::warn!(
                "no road near ({:.0}, {:.0}) for player {}; joining the lead",
                request.near[0],
                request.near[1],
                guest.id.0 + 1
            );
            drive.respawn_near([at.x, at.y], None);
        }
    }
}
