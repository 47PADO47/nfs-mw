//! The rival challenge in the flow: accepting the challenge button of the Blacklist leaves the safe house for the
//! world, which starts the bin's first boss race as a career race. The gauntlet goes on through that race's
//! `PostRaceActivity` (`docs/specs/story-flow-2.md` sections 4.3 and 4.4).

use nfsmw_career::blacklist::Blacklist;

use crate::app::Host;
use crate::frontend::logic::{Env, QuitTo};

use super::Frontend;
use super::flow_loading::LoadingScreen;

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
        self.save_settings(env.changed);
        self.career.set_prelude(false);
        self.leave_safe_house();
        self.origin = QuitTo::Career;
        self.quick_race = Some(race);
        self.start_free_roam(LoadingScreen::Tips, host, env);
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
