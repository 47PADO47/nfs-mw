//! The prologue in the flow (`docs/specs/story-flow.md` sections 2.1 to 2.3 and 2.7). A new career and a resumed prologue
//! start the prologue race the chain is at, as a career race in the car that race forces. The car lot that the scripted
//! chase of `16.2.1` opens picks the car the drive to the first safe house is made in.

use nfsmw_career::settings::NodeStates;
use nfsmw_career::story::prologue::{PrologueResume, resume_point};
use nfsmw_data::gameplay::GameplayData;

use crate::app::Host;
use crate::career::race_event_id;
use crate::frontend::logic::{Env, QuitTo};
use crate::frontend::look::CarLook;
use crate::scenes::world::{DEFAULT_CAR, RaceOptions};

use super::Frontend;
use super::flow_loading::LoadingScreen;

/// The prologue race a new career starts with (`16.1.0`, section 2.3 rule 1).
pub(super) const FIRST_PROLOGUE_RACE: &str = "16.1.0";

/// The car a race forces on the player, as the world names a car (`folder:preset`): the stock E46 GTR folder with the
/// race's preset ride, `RaceStory.player_car_type` (`M3GTRCAREERSTART` on all six prologue races). `None` when the race
/// forces no car or is not in the data.
pub(super) fn forced_car(gameplay: &GameplayData, event: &str) -> Option<String> {
    let race = gameplay.races.iter().find(|race| race_event_id(race) == event)?;
    let preset = race.story.player_car_type.as_deref()?;
    Some(format!("{DEFAULT_CAR}:{preset}"))
}

/// The prologue event a save loads into (section 2.7), or `None` when the save does not restart the prologue. The
/// career is being played (`begin_play`) when this runs.
pub(super) fn prologue_resume(nodes: &NodeStates, stage: u8) -> Option<&'static str> {
    match resume_point(stage, nodes, false) {
        PrologueResume::StartAt(event) => Some(event),
        PrologueResume::Done => None,
    }
}

impl Frontend {
    /// Starts the prologue race `event` behind the loading screen as a career race: the world is queued with it and the
    /// car the race forces, and the race's own chain takes over from its `PostRaceActivity`.
    pub(super) fn start_prologue_race(&mut self, event: &str, host: &mut Host, env: &mut Env) {
        let car = self.career.with(|career| forced_car(&career.data.gameplay, event)).flatten();
        log::info!("career: starting prologue race {event} in {car:?}");
        self.origin = QuitTo::Career;
        self.quick_race = Some(event.to_owned());
        self.race_options = Some(RaceOptions { career: true, ..RaceOptions::default() });
        self.quick_car = car;
        self.start_free_roam(LoadingScreen::Tips, host, env);
    }

    /// The prologue event a loaded career resumes at, or `None` when it does not restart (`prologue_resume`).
    pub(super) fn resume_prologue_event(&self) -> Option<&'static str> {
        self.career.with(|career| prologue_resume(&career.settings.nodes(), career.stage())).flatten()
    }

    /// The car lot after the scripted chase closed with Accept on the car the player picked: the world starts the drive to
    /// the first safe house in that car, through the loading screen. Leaving the car lot is a roaming start, as leaving the
    /// safe house is.
    pub(super) fn start_first_drive(&mut self, host: &mut Host, env: &mut Env) {
        let (folder, preset) = CarLook::of_profile(&self.career).resolve(&self.dir);
        log::info!("career: driving to the first safe house in {folder}");
        self.in_safe_house = true;
        self.leave_safe_house();
        self.origin = QuitTo::Career;
        self.quick_car = Some(match preset {
            Some(preset) => format!("{folder}:{preset}"),
            None => folder,
        });
        self.drive_to_safe_house = true;
        self.start_free_roam(LoadingScreen::Tips, host, env);
    }
}

#[cfg(test)]
#[path = "flow_prologue_tests.rs"]
mod tests;
