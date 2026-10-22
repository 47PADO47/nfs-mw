//! The co-op career's entry in the flow: the career car select's second car is player 2's, then the career resumes with
//! player 2 in the session. Player 1 keeps the keyboard, or the arrow block when player 2 took the keyboard. The world
//! brings player 2 in (`flow_loading.rs`), and the seats are kept while it has the window (`flow_split.rs`). Spec:
//! `docs/specs/coop-career.md` section 3.

use super::*;
use crate::frontend::custom_race::state as custom_state;
use crate::frontend::look::CarLook;
use crate::input::{CoopDevice, PlayerDevice};
use crate::scenes::world::players::coop::CoopSetup;
use crate::settings::Transmission;

impl Frontend {
    /// Player 2's car is picked in the career car select: the co-op session is set up, then the career resumes as
    /// `ResumeCareer` does, with player 2 in it. The device is the one player 2 pressed Start on.
    pub(super) fn coop_second_car(&mut self, handle: u32, transmission: Transmission, host: &mut Host, env: &mut Env) {
        // Player 2's car is found as player 1's is: the lot's name, a preset ride or the car type (`look_resolve.rs`).
        let look =
            self.career.with_loaded(|career| CarLook::of_handle(career, handle)).unwrap_or_else(CarLook::default_car);
        let folder = match look.resolve(&self.dir) {
            (folder, Some(preset)) => format!("{folder}:{preset}"),
            (folder, None) => folder,
        };
        let seats = custom_state::with(|state| state.split.seated());
        let alias = self.career.with_loaded(|career| career.alias.clone()).unwrap_or_else(|| "Player".into());
        let lead = self.career.with_loaded(|career| career.settings.car_handle);
        let shared = lead == Some(handle);
        log::info!(
            "co-op: player 2 drives {folder} (car {handle:#x}{})",
            if shared { ", shared with player 1" } else { "" }
        );
        self.coop = Some(coop_setup(alias, folder, handle, transmission, guest_device(seats)));
        crate::career::set_coop_players(2);
        self.resume_career(host, env);
    }

    /// The pause menu's End Co-op: player 2's car leaves the world, the front end's co-op state goes too (player 2's device
    /// is unseated and the count is one again), and play resumes.
    pub(super) fn end_coop_from_pause(&mut self, host: &mut Host, env: &mut Env) {
        host.scene.end_coop();
        self.end_coop();
        self.resume_driving(env);
    }

    /// Co-op is forgotten: the session ends and the count of players goes back to one.
    pub(super) fn end_coop(&mut self) {
        if self.coop.take().is_some() {
            crate::career::set_coop_players(1);
        }
    }
}

/// The pause menu's co-op rows: End Co-op while a session is on, greyed out while player 2 races or is chased.
pub(super) fn coop_pause(host: &Host) -> PauseCtx {
    let coop = crate::career::coop_players() > 1;
    PauseCtx { coop, coop_locked: host.scene.coop_end_locked(), ..PauseCtx::default() }
}

/// The second player of a co-op session, from the career car select's answer.
pub fn coop_setup(
    alias: String,
    folder: String,
    handle: u32,
    transmission: Transmission,
    device: CoopDevice,
) -> CoopSetup {
    CoopSetup { car: folder, transmission, alias, car_handle: Some(handle), device }
}

/// The device the guest drives with, from the seats of the Press Start screens: a pad when player 2 pressed a pad's Start,
/// otherwise the arrow block. Player 1 is not asked: `PressStart::seat` gives them the keyboard.
pub fn guest_device(seats: Option<[PlayerDevice; 2]>) -> CoopDevice {
    match seats {
        Some([_, PlayerDevice::Pad(_)]) => CoopDevice::Pad,
        _ => CoopDevice::Arrows,
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::Entity;

    use super::*;

    #[test]
    fn coop_setup_names_the_alias_and_car() {
        let setup = coop_setup("Pietro".into(), "M3GTR".into(), 7, Transmission::Manual, CoopDevice::Pad);
        assert_eq!(setup.alias, "Pietro");
        assert_eq!(setup.car, "M3GTR");
        assert_eq!(setup.car_handle, Some(7));
        assert_eq!(setup.transmission, Transmission::Manual);
        assert_eq!(setup.device, CoopDevice::Pad);
    }

    #[test]
    fn the_guest_drives_on_a_pad_only_when_player_two_pressed_one() {
        let pad = Entity::from_raw_u32(3).unwrap();
        assert_eq!(guest_device(None), CoopDevice::Arrows, "no seats, the arrow block");
        assert_eq!(guest_device(Some([PlayerDevice::Keyboard, PlayerDevice::KeyboardArrows])), CoopDevice::Arrows);
        assert_eq!(guest_device(Some([PlayerDevice::KeyboardArrows, PlayerDevice::Keyboard])), CoopDevice::Arrows);
        assert_eq!(guest_device(Some([PlayerDevice::Keyboard, PlayerDevice::Pad(pad)])), CoopDevice::Pad);
    }
}
