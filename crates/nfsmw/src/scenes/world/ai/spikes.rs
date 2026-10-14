//! Spike strips on the road: the player's tire that runs over a strip is punctured, and the tire model's blow-out
//! follows (`blackbox_vehicle` tires). The geometry and the test are in `blackbox_pursuit::spikes`. Spec:
//! `docs/specs/ai-pursuit-roadblocks.md` section 6. Only the player's car is spiked here: the racers' spiking is not built.

use blackbox_pursuit::spikes;

use super::TrafficWorld;
use crate::scenes::world::drive::CarSim;

impl TrafficWorld {
    /// Punctures each grounded tire of the player that is over a strip. A tire already punctured is left alone.
    pub(super) fn spike_player(&mut self, player: &mut CarSim) {
        if self.spikes.is_empty() {
            return;
        }
        let wheels = player.wheel_patches();
        for (wheel, hit) in spikes::punctured(&self.spikes, &wheels).into_iter().enumerate() {
            if hit && player.puncture_wheel(wheel) {
                log::info!("spike strip: tire {wheel} punctured");
            }
        }
    }

    /// Takes the spike strips away: the chase that laid them is over.
    pub(super) fn clear_spikes(&mut self) {
        self.spikes.clear();
    }
}
