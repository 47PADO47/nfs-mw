//! Capture readiness includes the end of a driver script; benchmark readiness includes its movement.

use super::WorldScene;

impl WorldScene {
    pub(super) fn loaded_for_bench(&self) -> bool {
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
