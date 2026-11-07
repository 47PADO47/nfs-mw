//! The career pause shortcut enters the same safe-house menu as a world zone. While a pursuit exists (chasing or cooling
//! down) the original's pause menu has neither Jump to Safe House nor any way into the Blacklist, whose pages start
//! chases, races and rival challenges: the rewrite leaves both out then (`docs/specs/frontend-menus.md`, "Pause menu").

use crate::app::Host;
use crate::frontend::logic::{Env, PauseCtx};

use super::{Frontend, Stage};

impl Frontend {
    pub(super) fn pause_context(&self, host: &Host) -> PauseCtx {
        let race = host.scene.race_active();
        let pursuit = host.scene.pursuit_feed().is_some();
        PauseCtx {
            race,
            quit: self.origin,
            blacklist: self.pause_blacklist(race).filter(|_| !pursuit),
            tuning: host.scene.tuning_available().filter(|_| !self.series.active && self.split.is_none()),
            safe_house: self.pause_safe_house(host),
            pursuit,
            ..super::flow_coop::coop_pause(host)
        }
    }

    pub(super) fn pause_safe_house(&self, host: &Host) -> Option<bool> {
        self.pause_blacklist(host.scene.race_active())?;
        if host.scene.pursuit_feed().is_some() {
            return None;
        }
        // Co-op room entry is not supported.
        Some(host.scene.coop_second_device().is_none())
    }

    pub(super) fn jump_to_safe_house(&mut self, host: &mut Host, env: &mut Env) {
        if self.stage != Stage::Paused || self.pause_safe_house(host) != Some(true) {
            return;
        }
        if !host.scene.prepare_safe_house() {
            return;
        }
        self.save_settings(env.changed);
        self.career.set_prelude(true);
        self.paused.set(false);
        self.open_safe_house(host, env);
    }
}

#[cfg(test)]
#[path = "flow_safehouse_tests.rs"]
mod tests;
