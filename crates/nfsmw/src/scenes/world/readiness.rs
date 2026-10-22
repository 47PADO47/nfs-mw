use super::WorldScene;
impl WorldScene {
    pub(super) fn ready_for_capture(&self) -> bool {
        self.ready_for_benchmark() && self.players.lead().is_none_or(|d| d.script_finished())
    }

    pub(super) fn ready_for_benchmark(&self) -> bool {
        if self.traffic.as_ref().is_some_and(|t| !t.settled()) {
            return false;
        }
        match self.players.lead() {
            Some(drive) => self.residency.complete() && drive.waiting_for_road().is_none(),
            None if self.wait_for_load => self.grounded && self.residency.complete(),
            None => self.residency.counts().2,
        }
    }
}
