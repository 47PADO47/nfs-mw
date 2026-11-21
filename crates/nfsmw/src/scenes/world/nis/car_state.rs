//! What a scene's car events set on the real car besides its pose: the lights (`ENISLights`) and the repairs
//! (`ENISCarDamageReset`, a repairing `ENISReattach`). Spec: `docs/specs/nis-events.md` section 2.2.

use super::car_events::CarMotion;
use super::cars::Target;
use crate::scenes::world::WorldScene;

impl WorldScene {
    /// Gives the car of `target` the lights its channel's events set and the repairs they asked for since the last
    /// frame. Nothing when its channel has no motion (no scene plays).
    pub(super) fn apply_nis_car_state(&mut self, target: Target, motion: Option<CarMotion>) {
        let Some(motion) = motion else { return };
        let repair = self.nis.car_memory().is_some_and(|memory| memory.take_repair(target, motion.repairs));
        match target {
            Target::Player => {
                let Some(drive) = self.players.lead_mut() else { return };
                drive.set_nis_headlights(motion.lights.map(|(_, head)| head));
                if repair {
                    drive.repair();
                }
            }
            Target::Traffic(index) | Target::Spawned(index) => {
                let Some(traffic) = self.traffic.as_mut() else { return };
                traffic.set_nis_lights(index, motion.lights);
                if repair {
                    traffic.repair_nis_car(index);
                }
            }
        }
    }
}
