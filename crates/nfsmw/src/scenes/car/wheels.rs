//! Moving wheels: the car's wheels and brakes are posed again every frame, so they spin and the front pair
//! steers. The instances keep their keys, so the renderer sees the same parts moving.

use glam::Mat4;
use nfsmw_data::car::WheelPose;

use super::CarScene;

impl CarScene {
    /// Turn the wheels: `spin` is the roll in radians, `steer` the front wheels' angle (positive turns left).
    /// The rear wheels spin but do not steer.
    pub(crate) fn pose_wheels(&mut self, spin: f32, steer: f32) {
        let Some(corners) = self.model.corners else { return };
        let world = self.to_world();
        let Self { model, instances, .. } = self;
        for instance in instances.iter_mut() {
            let (_, part) = instance.key.parts();
            let Some(placement) = model.placements.get(part as usize) else { continue };
            let Some(corner) = placement.corner else { continue };
            let pose = WheelPose { steer: if corner < 2 { steer } else { 0.0 }, spin, travel: 0.0 };
            let (wheel, brake) = corners[corner].posed(pose);
            let local: Mat4 = if placement.is_brake() { brake } else { wheel };
            instance.transform = world * local;
        }
    }
}
