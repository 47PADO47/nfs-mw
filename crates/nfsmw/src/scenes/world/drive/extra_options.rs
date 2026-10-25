//! Local testing controls do not alter the installed vehicle specification.
use super::Drive;
impl Drive {
    pub(in crate::scenes::world) fn configure_extra(&mut self, camera: &crate::extra_options::CameraPreferences) {
        if self.chase.preferences != *camera {
            self.chase.preferences = camera.clone();
        }
    }
    pub(in crate::scenes::world) fn center_camera(&mut self) {
        self.chase.center();
    }
    pub(in crate::scenes::world) fn refill_extra(&mut self, nos: bool, breaker: bool) {
        if nos && let Some(sim) = self.sim.as_mut() {
            sim.refill_nitrous();
            self.telemetry.nos = sim.telemetry().nos;
        }
        if breaker {
            self.breaker.meter = 1.0;
        }
    }
}
