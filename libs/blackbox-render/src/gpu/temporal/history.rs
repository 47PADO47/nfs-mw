//! Last frame's transform of every keyed instance, for the motion vectors of moving objects.

use std::collections::HashMap;
use std::ops::Range;

use blackbox_gfx::{Instance, InstanceKey};
use foldhash::fast::FixedState;
use glam::Mat4;

use crate::gpu::instances::InstanceBuffer;

/// A matrix as the bits the GPU holds, so a `-0.0` replacing a `0.0` still counts as a change.
type Bits = [u32; 16];

/// Unchanged matrices between two changed ones that are written along with them: one upload of a few extra
/// matrices is cheaper than a second `write_buffer` call.
const MAX_GAP: usize = 4;

type Transforms = HashMap<InstanceKey, Mat4, FixedState>;

pub(super) struct History {
    /// The transforms of the previous temporal frame.
    previous: Transforms,
    /// The transforms of the frame being drawn; becomes `previous` once it is.
    current: Transforms,
    /// Per instance, in the order of the frame's instances: the matrix it had in the previous frame.
    pub(super) buffer: InstanceBuffer,
    scratch: Vec<Bits>,
    /// What `buffer` holds from the last upload, as bits.
    uploaded: Vec<Bits>,
    /// The ranges of `scratch` that differ from `uploaded`.
    runs: Vec<Range<usize>>,
    /// Per instance: whether its transform differs from the one it had in the previous frame.
    moved: Vec<bool>,
}

impl History {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self {
            previous: Transforms::default(),
            current: Transforms::default(),
            buffer: InstanceBuffer::new(device),
            scratch: Vec::new(),
            uploaded: Vec::new(),
            runs: Vec::new(),
            moved: Vec::new(),
        }
    }

    /// Upload the previous matrix of each of `instances`, and remember their current ones for the next
    /// frame. An instance with no key, or one the previous frame did not have, stays where it is, so it
    /// only moves with the camera. After a `reset` nothing from before is trusted. Only the matrices that
    /// differ from what the buffer already holds are written.
    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, instances: &[Instance], reset: bool) {
        self.scratch.clear();
        self.moved.clear();
        self.current.clear();
        for instance in instances {
            let was = before(&self.previous, instance, reset);
            self.moved.push(was != instance.transform);
            self.scratch.push(was.to_cols_array().map(f32::to_bits));
            if !instance.key.is_transient() {
                self.current.insert(instance.key, instance.transform);
            }
        }
        std::mem::swap(&mut self.previous, &mut self.current);
        if self.buffer.reserve(device, self.scratch.len()) {
            self.uploaded.clear();
        }
        changed_runs(&self.uploaded, &self.scratch, &mut self.runs);
        for run in &self.runs {
            self.buffer.write_at(queue, run.start, &self.scratch[run.clone()]);
        }
        std::mem::swap(&mut self.uploaded, &mut self.scratch);
    }

    /// Per instance of the last [`upload`](Self::upload): whether it moved since the previous frame. The
    /// others have only the camera's motion.
    pub(super) fn moved(&self) -> &[bool] {
        &self.moved
    }

    /// Drop what is remembered: the next frame starts from the transforms it is given.
    pub(super) fn forget(&mut self) {
        self.previous.clear();
    }
}

/// Where `instance` was in the previous frame: its remembered transform, or its current one when it has no
/// key, was not there, or the history is not to be trusted.
fn before(previous: &Transforms, instance: &Instance, reset: bool) -> Mat4 {
    if reset || instance.key.is_transient() {
        return instance.transform;
    }
    previous.get(&instance.key).copied().unwrap_or(instance.transform)
}

/// Fill `runs` with the ranges of `new` to write so a buffer holding `old` ends up holding `new`: every matrix
/// that differs (or has no counterpart in `old`), with runs closer than [`MAX_GAP`] merged.
fn changed_runs(old: &[Bits], new: &[Bits], runs: &mut Vec<Range<usize>>) {
    runs.clear();
    for (index, matrix) in new.iter().enumerate() {
        if old.get(index) == Some(matrix) {
            continue;
        }
        if let Some(last) = runs.last_mut().filter(|last| index <= last.end + MAX_GAP) {
            last.end = index + 1;
            continue;
        }
        runs.push(index..index + 1);
    }
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::MeshHandle;
    use glam::Vec3;

    use super::*;

    fn at(x: f32) -> Mat4 {
        Mat4::from_translation(Vec3::X * x)
    }

    fn map(entries: &[(InstanceKey, Mat4)]) -> Transforms {
        entries.iter().copied().collect()
    }

    #[test]
    fn a_keyed_instance_remembers_where_it_was() {
        let key = InstanceKey::new(1, 2);
        let instance = Instance::keyed(MeshHandle::from_raw(1), at(5.0), key);
        let previous = map(&[(key, at(4.0))]);
        assert_eq!(before(&previous, &instance, false), at(4.0));
    }

    #[test]
    fn a_new_transient_or_reset_instance_stays_where_it_is() {
        let key = InstanceKey::new(1, 2);
        let keyed = Instance::keyed(MeshHandle::from_raw(1), at(5.0), key);
        let transient = Instance::new(MeshHandle::from_raw(1), at(5.0));
        let previous = map(&[(key, at(4.0)), (InstanceKey::TRANSIENT, at(1.0))]);
        assert_eq!(before(&Transforms::default(), &keyed, false), at(5.0), "it was not there last frame");
        assert_eq!(before(&previous, &transient, false), at(5.0), "no identity, no history");
        assert_eq!(before(&previous, &keyed, true), at(5.0), "a camera cut trusts nothing");
    }

    fn bits(value: u32) -> Bits {
        [value; 16]
    }

    #[test]
    fn only_changed_matrices_are_written() {
        let old = [bits(1), bits(2), bits(3)];
        let mut runs = Vec::new();
        changed_runs(&old, &old, &mut runs);
        assert!(runs.is_empty());
        changed_runs(&old, &[bits(1), bits(9), bits(3)], &mut runs);
        assert_eq!(runs, vec![1..2]);
    }

    #[test]
    fn new_matrices_past_the_old_end_are_written() {
        let mut runs = Vec::new();
        changed_runs(&[bits(1)], &[bits(1), bits(2), bits(3)], &mut runs);
        assert_eq!(runs, vec![1..3]);
        changed_runs(&[], &[bits(1), bits(2)], &mut runs);
        assert_eq!(runs, vec![0..2]);
    }

    #[test]
    fn near_runs_merge_and_far_ones_stay_apart() {
        let old = vec![bits(0); 20];
        let mut new = old.clone();
        new[2] = bits(1);
        new[5] = bits(1);
        new[15] = bits(1);
        let mut runs = Vec::new();
        changed_runs(&old, &new, &mut runs);
        assert_eq!(runs, vec![2..6, 15..16]);
    }
}
