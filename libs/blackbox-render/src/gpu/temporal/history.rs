//! Last frame's transform of every keyed instance, for the motion vectors of moving objects.

use std::collections::HashMap;

use blackbox_gfx::{Instance, InstanceKey};
use glam::Mat4;

use crate::gpu::instances::InstanceBuffer;

pub(super) struct History {
    /// The transforms of the previous temporal frame.
    previous: HashMap<InstanceKey, Mat4>,
    /// The transforms of the frame being drawn; becomes `previous` once it is.
    current: HashMap<InstanceKey, Mat4>,
    /// Per instance, in the order of the frame's instances: the matrix it had in the previous frame.
    pub(super) buffer: InstanceBuffer,
    scratch: Vec<[f32; 16]>,
}

impl History {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            previous: HashMap::new(),
            current: HashMap::new(),
            buffer: InstanceBuffer::new(device),
            scratch: Vec::new(),
        }
    }

    /// Upload the previous matrix of each of `instances`, and remember their current ones for the next
    /// frame. An instance with no key, or one the previous frame did not have, stays where it is, so it
    /// only moves with the camera. After a `reset` nothing from before is trusted.
    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, instances: &[Instance], reset: bool) {
        self.scratch.clear();
        self.current.clear();
        for instance in instances {
            self.scratch.push(before(&self.previous, instance, reset).to_cols_array());
            if !instance.key.is_transient() {
                self.current.insert(instance.key, instance.transform);
            }
        }
        std::mem::swap(&mut self.previous, &mut self.current);
        self.buffer.upload(device, queue, &self.scratch);
    }

    /// Drop what is remembered: the next frame starts from the transforms it is given.
    pub(super) fn forget(&mut self) {
        self.previous.clear();
    }
}

/// Where `instance` was in the previous frame: its remembered transform, or its current one when it has no
/// key, was not there, or the history is not to be trusted.
fn before(previous: &HashMap<InstanceKey, Mat4>, instance: &Instance, reset: bool) -> Mat4 {
    if reset || instance.key.is_transient() {
        return instance.transform;
    }
    previous.get(&instance.key).copied().unwrap_or(instance.transform)
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::MeshHandle;
    use glam::Vec3;

    use super::*;

    fn at(x: f32) -> Mat4 {
        Mat4::from_translation(Vec3::X * x)
    }

    #[test]
    fn a_keyed_instance_remembers_where_it_was() {
        let key = InstanceKey::new(1, 2);
        let instance = Instance::keyed(MeshHandle::from_raw(1), at(5.0), key);
        let previous = HashMap::from([(key, at(4.0))]);
        assert_eq!(before(&previous, &instance, false), at(4.0));
    }

    #[test]
    fn a_new_transient_or_reset_instance_stays_where_it_is() {
        let key = InstanceKey::new(1, 2);
        let keyed = Instance::keyed(MeshHandle::from_raw(1), at(5.0), key);
        let transient = Instance::new(MeshHandle::from_raw(1), at(5.0));
        let previous = HashMap::from([(key, at(4.0)), (InstanceKey::TRANSIENT, at(1.0))]);
        assert_eq!(before(&HashMap::new(), &keyed, false), at(5.0), "it was not there last frame");
        assert_eq!(before(&previous, &transient, false), at(5.0), "no identity, no history");
        assert_eq!(before(&previous, &keyed, true), at(5.0), "a camera cut trusts nothing");
    }
}
