//! Persist the loaded profile's option without checkpointing its unsaved rule state.
use anyhow::Result;
use nfsmw_career::save::ProfileSave;

use crate::career::{Career, ProfileDir};

impl Career {
    pub fn set_autosave_enabled(&mut self, profiles: &ProfileDir, enabled: bool) -> Result<()> {
        let mut stored = match profiles.path(&self.alias).try_exists()? {
            true => ProfileSave::parse(&profiles.read(&self.alias)?)?,
            false => self.profile.clone(),
        };
        stored.set_autosave_enabled(enabled);
        profiles.write(&self.alias, &stored.to_bytes())?;
        self.profile.set_autosave_enabled(enabled);
        self.autosave.option_on = enabled;
        self.autosave.pending = false;
        self.autosave.armed = false;
        self.autosave.off_by_error = false;
        self.autosave.re_enable = false;
        Ok(())
    }
}
