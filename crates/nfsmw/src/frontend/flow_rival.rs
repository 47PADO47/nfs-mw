//! The rival challenge in the flow: accepting the challenge button of the Blacklist leaves the safe house for the
//! world, which starts the bin's first boss race as a career race. The gauntlet goes on through that race's
//! `PostRaceActivity` (`docs/specs/story-flow-2.md` sections 4.3 and 4.4).

use nfsmw_career::blacklist::Blacklist;

use crate::app::Host;
use crate::frontend::logic::{Env, QuitTo};

use super::Frontend;
use super::flow_loading::LoadingScreen;
use crate::frontend::flow::Stage;

impl Frontend {
    /// The challenge button of bin `bin` was accepted: the world starts the bin's first boss race behind the loading
    /// screen. Without such a race nothing changes.
    pub(super) fn challenge_rival(&mut self, bin: u8, host: &mut Host, env: &mut Env) {
        let race = self.career.with(|career| first_boss_race(&career.blacklist, bin)).flatten();
        let Some(race) = race else {
            log::warn!("career: bin {bin} has no boss race to challenge");
            return;
        };
        log::info!("career: challenging the rival of bin {bin} with {race}");
        self.leave_safe_house_for_race(&race, host, env);
    }

    /// A race of the race events sheet was accepted: the world starts the career event `event` behind the loading screen.
    pub(super) fn launch_event(&mut self, event: String, host: &mut Host, env: &mut Env) {
        log::info!("career: launching event {event} from the race events sheet");
        self.leave_safe_house_for_race(&event, host, env);
    }

    /// A milestone or a bounty place of the sheets was accepted: the world starts with the car at `at` (render space).
    pub(super) fn jump_to(&mut self, at: [f32; 2], host: &mut Host, env: &mut Env) {
        log::info!("career: jumping to ({:.0}, {:.0}) from the rap sheet", at[0], at[1]);
        // From the pause menu the world is already there: the car goes to the point and the game goes on.
        if self.stage == Stage::Paused {
            if host.scene.jump_player(at) {
                self.resume();
            }
            return;
        }
        self.save_settings(env.changed);
        self.career.set_prelude(false);
        self.leave_safe_house();
        self.origin = QuitTo::Career;
        self.jump_start = Some(at);
        self.start_free_roam(LoadingScreen::Tips, host, env);
    }

    /// The stage the pause menu's Blacklist item opens at: the career's bin while it is played and no race runs, once the
    /// first rival is entered (the prologue has no Blacklist).
    pub(super) fn pause_blacklist(&self, race: bool) -> Option<u8> {
        if race || self.origin != QuitTo::Career {
            return None;
        }
        self.career.with(|career| career.stage()).filter(|stage| (1..=15).contains(stage))
    }

    /// Leaves the safe house for the world, which runs `race` as a career race (its result counts in the Blacklist).
    fn leave_safe_house_for_race(&mut self, race: &str, host: &mut Host, env: &mut Env) {
        // From the pause menu the race starts in the running world.
        if self.stage == Stage::Paused {
            if host.scene.start_career_event(race) {
                self.resume();
            }
            return;
        }
        self.save_settings(env.changed);
        self.career.set_prelude(false);
        self.leave_safe_house();
        self.start_career_race(race, host, env);
    }
}

/// The event id of the first boss race of bin `bin`, the race a challenge starts. `None` when the Blacklist has no
/// such bin or the bin has no boss race.
fn first_boss_race(blacklist: &Blacklist, bin: u8) -> Option<String> {
    blacklist.bin(bin)?.def.rival_events.first().cloned()
}

#[cfg(test)]
#[path = "flow_rival_tests.rs"]
mod tests;
