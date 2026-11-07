//! The rider of a bike on show: drawn on the bike in the car viewer and the garage's backdrop, in the gear on show there
//! (`docs/specs/ride6-rider-gear.md`). The rider is the world's (`scenes/world/drive/rider`); here he sits on the bike at rest, in the
//! pose the animation gives a bike that is not moving.

use blackbox_gfx::RenderBackend;

use super::{CarScene, Instance};
use crate::garage::rider_gear;
use crate::scenes::world::drive::CarPose;
use crate::scenes::world::drive::rider::Rider;

impl CarScene {
    /// Where the bike stands, as the pose the rider is placed from.
    fn rider_pose(&self) -> CarPose {
        let (_, rotation, position) = self.to_world().to_scale_rotation_translation();
        CarPose { position, rotation, wheels: [Default::default(); 4] }
    }

    /// Once per update: load the rider of a bike mod car the first time it is shown, dress him in the gear on show, pose and skin
    /// him, and list his parts after the car's. A car, a bike without rider files, and the setting `rider_gear` off, show no rider.
    pub(super) fn maintain_rider(&mut self, renderer: &mut dyn RenderBackend) {
        self.instances.truncate(self.car_instances);
        let Some(source) = &self.source else { return };
        let bike = nfsmw_data::car::mods::bike::is_bike(&self.model.name);
        if !bike || !rider_gear::enabled() {
            return;
        }
        let pose = self.rider_pose();
        self.rider.maintain(renderer, &source.dir, &self.model, (&pose, true));
        let mut rider: Vec<Instance> = Vec::new();
        self.rider.instances(&pose, &mut rider);
        self.instances.extend(rider);
    }

    /// Frees the rider's meshes and pictures; the next update loads him again.
    pub(super) fn release_rider(&mut self, renderer: &mut dyn RenderBackend) {
        std::mem::replace(&mut self.rider, Rider::previewing()).release(renderer);
    }
}
