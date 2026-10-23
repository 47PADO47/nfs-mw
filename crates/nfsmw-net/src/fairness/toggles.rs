//! MP-34 nitrous and damage toggles. The server applies them: with nitrous off it clears the nitrous button of every
//! input before the world steps, so a client cannot burn nitrous by sending the bit. Damage stays visual (features.md);
//! with damage off the world shows none. Repair in the lobby clears damage between races.
//!
//! Choice **[ours]**: repair in the lobby is on by default; the settings struct has no field for it yet.

use blackbox_vehicle::InputState;

use crate::message::{DamageMode, Settings};

/// The session's nitrous and damage rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Toggles {
    pub nitrous: bool,
    pub damage: DamageMode,
    /// Cars are repaired when the session returns to the lobby.
    pub repair_in_lobby: bool,
}

impl Default for Toggles {
    /// Nitrous on, visual damage, repair in the lobby.
    fn default() -> Self {
        Self { nitrous: true, damage: DamageMode::Visual, repair_in_lobby: true }
    }
}

impl Toggles {
    /// The toggles of `settings`.
    pub fn from_settings(settings: &Settings) -> Self {
        Self { nitrous: settings.nitrous, damage: settings.damage, ..Self::default() }
    }

    /// The input as the world may use it: the nitrous button cleared when nitrous is off.
    pub fn filter_input(&self, input: &InputState) -> InputState {
        let mut out = *input;
        if !self.nitrous {
            out.nos = false;
        }
        out
    }

    /// Whether damage shows on cars.
    pub fn damage_shows(&self) -> bool {
        self.damage == DamageMode::Visual
    }
}
