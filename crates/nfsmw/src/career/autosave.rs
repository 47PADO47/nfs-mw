//! The autosave at run time: asks the rule crate's [`AutosaveManager`] whether a trigger saves now and writes the
//! profile when it does. The gate, the pending / armed bits and the failure handling are the rule crate's; this file
//! only feeds it the run-time facts and performs the write. Spec: `docs/specs/career-save-screens-2.md` section 8
//! and the rewrite rules of section 14.

use nfsmw_career::safehouse::FrontEndContext;
use nfsmw_career::savefile::{CurrentNode, GateInputs, RunDecision, Trigger};

use super::{Career, CareerHandle, ProfileDir};

/// The run-time facts the autosave gate needs that only the caller knows. The profile autosave option itself is
/// not here: it lives in the career's [`AutosaveManager`] (default on), as the original keeps it in the profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutosaveInputs {
    /// The front-end context word. The online and LAN bits block the save.
    pub context: FrontEndContext,
    /// The gameplay manager's current node id (for example `14.2.1`), `None` when no event node is current.
    pub node_id: Option<String>,
    /// The current node's boolean attribute `BossRace`. Only read when `node_id` is set.
    pub boss_race: bool,
    /// The gameplay manager is idle: its notification list is empty, or no notification can be shown right now.
    pub gameplay_idle: bool,
    /// A profile is loaded (the profile loaded byte). A run does nothing without one.
    pub profile_loaded: bool,
}

impl AutosaveInputs {
    /// The rule crate's gate inputs for these facts: an unforced evaluation with the layer present (always true on
    /// PC).
    fn gate(&self) -> GateInputs<'_> {
        let current_node = self.node_id.as_deref().map(|id| CurrentNode { id, boss_race: self.boss_race });
        GateInputs { force: false, context: self.context, layer_present: true, current_node }
    }
}

/// Runs the autosave for `trigger`. Returns `true` when the profile was written.
///
/// The trigger posts the `EAutoSave` event only when [`Trigger::posts_event`] says so. The event is disposed at
/// once (it sets pending when the gate passes), the world's HUD update arms it, and the manager's tick starts the
/// run when the gameplay manager is idle. A failed write goes to the rule crate's failure handling, which turns the
/// profile autosave option off; the continue question is not shown yet (see the module's notes in the report).
pub fn autosave(
    career: &mut Career,
    profiles: &ProfileDir,
    trigger: Trigger<'_>,
    inputs: AutosaveInputs,
) -> anyhow::Result<bool> {
    if !trigger.posts_event() {
        return Ok(false);
    }
    let gate = inputs.gate();
    career.autosave.event_disposed(&gate);
    career.autosave.hud_update();
    let decision = career.autosave.tick(inputs.gameplay_idle, &gate, inputs.profile_loaded);
    match decision {
        None | Some(RunDecision::Nothing) => Ok(false),
        // "Switched off by error": the continue question needs a screen, which this path does not open yet.
        Some(RunDecision::AskContinue { .. }) => {
            bevy_log::info!("autosave waits for the continue question after a failed write");
            Ok(false)
        }
        Some(RunDecision::Write { .. }) => write_profile(career, profiles).map(|()| true),
    }
}

/// Serialises the career and writes it under its alias, then reports the outcome to the manager.
fn write_profile(career: &mut Career, profiles: &ProfileDir) -> anyhow::Result<()> {
    let bytes = career.profile_bytes();
    match profiles.save(&career.alias, &bytes) {
        Ok(()) => {
            career.autosave.write_completed();
            Ok(())
        }
        Err(err) => {
            // The returned prompt ("continue with autosave?") needs a screen; it is dropped until one exists.
            let _prompt = career.autosave.write_failed();
            Err(err)
        }
    }
}

/// Runs the autosave for the career in `handle`. Does nothing outside career mode. Errors are logged, never
/// returned: a failed autosave must not stop the game.
pub fn handle_autosave(
    handle: &CareerHandle,
    profiles: Option<&ProfileDir>,
    trigger: Trigger<'_>,
    inputs: AutosaveInputs,
) {
    if !handle.is_active() {
        return;
    }
    let profiles = match profiles {
        Some(profiles) => profiles,
        None => {
            bevy_log::warn!("autosave skipped: no profile folder");
            return;
        }
    };
    if let Some(Err(err)) = handle.with(|career| autosave(career, profiles, trigger, inputs)) {
        bevy_log::warn!("autosave failed: {err:#}");
    }
}

#[cfg(test)]
#[path = "autosave_tests.rs"]
mod tests;
