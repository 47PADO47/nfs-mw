//! Career on the main menu (`career-flow.md` sections 2 to 3): resume the latest profile when its career has started,
//! otherwise start a new career on a profile alias.

use std::sync::Arc;

use anyhow::Context;
use game_install::GameDir;
use nfsmw_career::settings::{CareerSettings, NewCareer};

use super::{Career, CareerData, ProfileDir};

/// The alias a new career is saved under when there is no profile yet. The original shows "Player 1" for an alias with
/// no name (`save-alias-manager.md` section 5), but a name with a space or of 8 characters or more is never listed
/// (`ProfileDir::aliases`), so the folder name is one the listing finds.
pub const DEFAULT_ALIAS: &str = "PLAYER";

/// What the main menu's Career does with the profiles: load the latest one, or start a new career on the default alias.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Load the profile with this alias; it is resumed if its career has started, else a new career takes its place.
    Load(String),
    /// No profile yet: a new career on this alias.
    New(String),
}

/// Picks the profile for Career from the latest profile's alias, if any. Pure, so the choice is testable without files.
pub fn choose(latest: Option<&str>) -> Choice {
    match latest {
        Some(alias) => Choice::Load(alias.to_owned()),
        None => Choice::New(DEFAULT_ALIAS.to_owned()),
    }
}

/// Starts or resumes the career for Career on the main menu. A profile whose career has not started (flags bit 0
/// clear) gets a new career on the same alias, as the original does (`career-flow.md` section 2).
pub fn start_or_resume(dir: &GameDir, profiles: &ProfileDir) -> anyhow::Result<Career> {
    let data = Arc::new(CareerData::load(dir).context("the career tables could not be read")?);
    match choose(profiles.latest().as_deref()) {
        Choice::Load(alias) => {
            let bytes = profiles.read(&alias).with_context(|| format!("the profile {alias} could not be read"))?;
            if career_started(&bytes)? {
                return Career::from_profile_bytes(&alias, &bytes, data);
            }
            new_career(&alias, data, profiles)
        }
        Choice::New(alias) => new_career(&alias, data, profiles),
    }
}

/// True when the profile's settings say the career has started (flags bit 0, `CAREER_STARTED`).
fn career_started(profile: &[u8]) -> anyhow::Result<bool> {
    let settings = CareerSettings::from_profile_bytes(profile).context("the profile settings could not be read")?;
    Ok(settings.has_career())
}

/// Makes a new career on `alias` and writes it to the profile folder. The UG2 bonus is not added: the folder check
/// belongs to the caller and is not made here yet.
fn new_career(alias: &str, data: Arc<CareerData>, profiles: &ProfileDir) -> anyhow::Result<Career> {
    let opts = NewCareer::default();
    let mut career =
        Career::new(alias, data, &opts).with_context(|| format!("the career {alias} could not be made"))?;
    profiles.write(alias, &career.profile_bytes()).with_context(|| format!("the career {alias} could not be saved"))?;
    Ok(career)
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod tests;
