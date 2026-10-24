//! Context-selected installed values, with standalone synthetic callers retaining their rule defaults.

use super::Police;
use nfsmw_data::pursuit::{LevelTuning, SupportRow};

impl Police {
    /// Installs INI settings while keeping an explicitly enabled F8 active-car override separate.
    pub fn set_settings(
        &mut self,
        settings: super::BartenderSettings,
        fairbusts: crate::settings::fairbusts::FairBustsSettings,
    ) {
        self.reset_penalty = fairbusts.penalty(settings.busted.reach);
        self.settings = settings;
        self.configured_max_cops = self.settings.max_cops;
        self.settings.max_cops = self.heat_model.cop_limit(self.configured_max_cops);
    }

    pub fn set_support_rows(&mut self, rows: Vec<SupportRow>) {
        self.support_rows = rows;
    }

    pub fn support_row(&self, level: u32) -> Option<&SupportRow> {
        self.support_rows.get(level.clamp(1, 10) as usize - 1)
    }

    pub fn set_level_tuning(&mut self, rows: Vec<LevelTuning>) {
        self.level_tuning = rows;
    }

    pub fn level_tuning(&self, level: u32) -> Option<LevelTuning> {
        self.level_tuning.get(level.clamp(1, 10) as usize - 1).copied()
    }
}

impl super::super::TrafficWorld {
    pub fn set_pursuit_options(&mut self, options: crate::settings::PursuitOptions) {
        self.police.set_pursuit_options(options);
    }
}
