use super::WorldScene;
use super::race::Stage;

impl WorldScene {
    /// A race is asked for (a quick race, a career race, the next race of a chain, a restart from the results) or has the
    /// car on its way to the grid: the start's scenery is not there yet, so the front end keeps the loading screen up
    /// (`docs/specs/loading-screen.md` section 6).
    pub(super) fn race_is_staging(&self) -> bool {
        match &self.race {
            Some(session) => matches!(session.stage, Stage::Loading { .. }),
            None => self.queued_race.is_some(),
        }
    }

    pub(super) fn ready_for_capture(&self) -> bool {
        self.ready_for_benchmark() && self.drive.as_ref().is_none_or(|d| d.script_finished())
    }

    pub(super) fn ready_for_benchmark(&self) -> bool {
        if self.traffic.as_ref().is_some_and(|t| !t.settled()) {
            return false;
        }
        match &self.drive {
            Some(drive) => self.residency.complete() && drive.waiting_for_road().is_none(),
            None if self.wait_for_load => self.grounded && self.residency.complete(),
            None => self.residency.counts().2,
        }
    }
}
