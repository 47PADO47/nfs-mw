//! The escape as the world feeds it: the cool-down steps with the cops' sight and the player's hiding, the end reason of
//! the chase, and the end card's clock. The rules are in `blackbox_pursuit::cooldown` and `blackbox_pursuit::evade`;
//! this file reads the per-frame sensing and applies the outcome. Child of `police`, so it reads the police state
//! directly. Spec: `docs/specs/ai-pursuit.md` sections 4.3, 6 and 7.

use super::*;
use blackbox_pursuit::CooldownStatus;
use blackbox_pursuit::cooldown::{CooldownInput, TUNING as COOLDOWN_TUNING};
use blackbox_pursuit::evade;

impl Police {
    pub(in crate::scenes::world::ai) fn sight_allowed(&self) -> bool {
        self.cooldown.can_show_busted(&COOLDOWN_TUNING)
    }

    /// Steps the cool-down one frame at the heat `level` and returns `true` on the frame the player got away. A busted
    /// chase does not tick it: the busted outcome comes first and the world ends the chase with it.
    pub(super) fn tick_cooldown(&mut self, dt: f32, sensed: Sensed, level: u32) -> bool {
        if self.busted.is_busted() {
            return false;
        }
        let observed_age = self.observation.ground_sight_age.map(|age| {
            match self.helicopter_sees() || self.observation.secondary_seen || self.final_pursuit() {
                true => age.min(0.0),
                false => age,
            }
        });
        let input = CooldownInput {
            seen: observed_age.is_none() && (sensed.seen || self.final_pursuit()),
            hidden: sensed.hidden,
            required: self.level_tuning(level).map_or_else(|| evade::timeout(level), |row| row.evade_timeout),
            hidden_multiplier: self
                .level_tuning(level)
                .map_or_else(|| evade::hidden_multiplier(level), |row| row.hidden_multiplier),
        };
        self.cooldown.update_observed(dt, &input, &COOLDOWN_TUNING, observed_age) == CooldownStatus::Evaded
    }

    /// Advances the end card's clock once the chase is over. Without an end reason there is no card, so nothing runs.
    pub(super) fn age_end_card(&mut self, dt: f32) {
        if self.end_reason == EndReason::None {
            return;
        }
        self.since_end += dt;
    }

    /// How the last chase ended (`None` while one runs, and before any chase).
    pub fn end_reason(&self) -> EndReason {
        self.end_reason
    }

    /// Seconds since the last chase ended, for the end card's fade.
    pub fn since_end(&self) -> f32 {
        self.since_end
    }

    /// The escape's status: the chase, the cool-down, or EVADED.
    #[allow(dead_code)] // read by the camera (E8f) and the tests, not built yet for the HUD
    pub fn cooldown_status(&self) -> CooldownStatus {
        self.cooldown.status()
    }

    /// The cool-down meter, 0 to 1, for the HUD's bar.
    pub fn evade_level(&self) -> f32 {
        self.cooldown.level()
    }

    /// The fill of the pursuit board's cool-down bar: empty when the cool-down opens, full at EVADED. This is not
    /// [`evade_level`](Self::evade_level), which already holds the grace's share of the required time.
    pub fn cooldown_bar(&self) -> f32 {
        self.cooldown.bar(&COOLDOWN_TUNING)
    }

    /// The escape's progress in seconds, for the audio feed (`time_since_any_saw` plus the hiding time).
    pub fn escape_progress(&self) -> f32 {
        self.cooldown.progress()
    }

    /// The seconds still to go before the player gets away, for the audio feed and the status line.
    pub fn secs_to_evade(&self) -> f32 {
        self.cooldown.remaining(&COOLDOWN_TUNING)
    }
}
