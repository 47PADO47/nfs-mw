//! Host-owned transitions also gate semantic input and its automatic sound feedback.

use blackbox_feng::{PackageId, ids};

use super::{Screens, command};
use crate::frontend::logic::Footer;
use crate::frontend::logic::{Command, Env};
use blackbox_feng::ids::BUTTON_PRESSED;
use blackbox_feng::runtime::Outgoing;

impl Screens {
    pub(super) fn dispatch_outgoing(&mut self, outgoing: Vec<Outgoing>, env: &mut Env, commands: &mut Vec<Command>) {
        let mut sounds = crate::audio::menu::FrameSounds::new(&outgoing);
        let mut input_sounds = Vec::new();
        for out in outgoing {
            match out {
                Outgoing::Game { message, package, .. } => self.deliver(package, message, env, commands),
                Outgoing::Package { kind, name, .. } => commands.extend(command::package_command(kind, name)),
                Outgoing::Sound { message, package, from, authored } => {
                    if authored {
                        self.queue_sound(&mut sounds, package, message, true);
                    }
                    if !authored {
                        input_sounds.push((package, message));
                    }
                    // Some buttons announce only through the sound system.
                    if let (BUTTON_PRESSED, Some(button)) = (message, from) {
                        self.deliver_press(package, button, env, commands);
                    }
                }
            }
        }
        for (package, message) in input_sounds {
            self.queue_sound(&mut sounds, package, message, false);
        }
    }

    pub(super) fn queue_sound(
        &mut self,
        sounds: &mut crate::audio::menu::FrameSounds,
        package: PackageId,
        message: u32,
        authored: bool,
    ) {
        if let Some(candidate) = sounds.candidate(package, message, authored)
            && let Some(id) = self.sound_trigger(package, message, candidate)
        {
            crate::audio::menu::request_trigger(id);
        }
    }

    pub(in crate::frontend) fn photo_audio_state(&self) -> Option<bool> {
        self.stack
            .iter()
            .find(|screen| screen.name.eq_ignore_ascii_case(crate::frontend::ids::screen::PHOTO))
            .map(|screen| screen.logic.holds_photo_audio_pause())
    }

    pub(super) fn top_accepts_input(&self) -> bool {
        !self.await_release && self.stack.last().is_some_and(|top| top.logic.accepts_input())
    }

    pub(super) fn sound_trigger(&mut self, package: PackageId, message: u32, cue: Option<i32>) -> Option<i32> {
        // An input fallback cannot add Back to the stock rival bio.
        if message == ids::PAD_BACK
            && self.stack.iter().any(|screen| {
                screen.id == package
                    && screen.name.eq_ignore_ascii_case(crate::frontend::ids::screen::SAFE_HOUSE_RIVAL_BIO)
            })
        {
            return None;
        }
        // Showcase directions operate its camera. Its authored responses do not request navigation sounds.
        let Some(screen) = self.stack.iter_mut().find(|screen| screen.id == package) else { return cue };
        if screen.logic.footer() == Footer::Orbit && direction(message) {
            return None;
        }
        screen.logic.sound_trigger(&self.runtime, package, message, cue)
    }
}

fn direction(message: u32) -> bool {
    matches!(
        message,
        ids::PAD_UP
            | ids::PAD_DOWN
            | ids::PAD_LEFT
            | ids::PAD_RIGHT
            | ids::PAD_UPLEFT
            | ids::PAD_UPRIGHT
            | ids::PAD_DOWNLEFT
            | ids::PAD_DOWNRIGHT
    )
}

/// Back is user input, even when a package broadcasts it to covered screens as part of its exit script.
pub(super) fn deliver_to_owner(top: Option<PackageId>, package: PackageId, message: u32) -> bool {
    message != ids::PAD_BACK || top == Some(package)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covered_screens_receive_lifecycle_events_but_not_back_input() {
        let covered = PackageId(1);
        let top = PackageId(2);
        assert!(!deliver_to_owner(Some(top), covered, ids::PAD_BACK));
        assert!(deliver_to_owner(Some(top), top, ids::PAD_BACK));
        assert!(deliver_to_owner(Some(top), covered, crate::frontend::ids::EXIT_COMPLETE));
        assert!(deliver_to_owner(Some(top), covered, crate::frontend::ids::INIT_COMPLETE));
    }
}
