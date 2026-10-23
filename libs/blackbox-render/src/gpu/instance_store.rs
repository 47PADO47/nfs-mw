//! The persistent instance store: every instance's model matrix and receiver metadata in one renderer-owned storage buffer, at a slot
//! that a keyed instance keeps from frame to frame. Each frame only the slots whose matrix changed are written,
//! and each view uploads a list of 4-byte slot indices instead of an 80-byte model and metadata record.
//!
//! Slots are handed out per epoch. An epoch starts with each scene frame (and each view capture), when every
//! encoder that used the store has been submitted, so a slot is written at most once while any one submission
//! is being recorded:
//! - a keyed instance keeps its slot while it is seen at least once every [`RETAIN_EPOCHS`] epochs;
//! - a transient instance ([`InstanceKey::TRANSIENT`]) takes a slot for this epoch only. The next epoch hands
//!   the same slots out again in the same order, so a steady stream of transient instances rewrites nothing;
//! - a keyed instance seen twice in one epoch with different matrices takes a transient slot the second time,
//!   so the first view keeps the matrix it was recorded with.
//!
//! A renderer without storage buffers in the vertex stage (the GL backend) keeps the per-view matrix upload.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::ops::Range;

use blackbox_gfx::{Instance, InstanceKey};

/// Epochs a keyed slot outlives the last frame its instance was drawn in.
pub(in crate::gpu) const RETAIN_EPOCHS: u32 = 120;
/// Unchanged slots a dirty range may span to join its neighbour: one larger write beats two small ones.
const JOIN_GAP: u32 = 4;
/// Slots the store holds before it first grows.
const INITIAL_SLOTS: u32 = 4096;
/// Bytes of one slot: a column-major 4x4 matrix and aligned receiver metadata.
const SLOT_BYTES: u64 = 80;

/// Whether `device` can read the store from vertex shaders. GL may lack vertex-stage storage buffers, and a
/// device created with WebGL2 limits has none at all.
pub(in crate::gpu) fn supported(device: &wgpu::Device) -> bool {
    if device.adapter_info().backend == wgpu::Backend::Gl {
        return false;
    }
    let limits = device.limits();
    limits.max_storage_buffers_per_shader_stage > 0
        && limits.max_storage_buffer_binding_size >= u64::from(INITIAL_SLOTS) * SLOT_BYTES
}

/// The bind group layout of the store: one read-only storage buffer of matrices, read by vertex shaders. Equal
/// layouts are interchangeable, so every pipeline that reads the store can make its own.
pub(in crate::gpu) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("instance store"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// A hasher for [`InstanceKey`]s, which are already well spread `u64`s: one multiply instead of SipHash, since
/// every instance of every view is looked up each frame.
#[derive(Default, Clone, Copy)]
pub(in crate::gpu) struct KeyHasher(u64);

impl Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0.rotate_left(8) ^ u64::from(byte)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }

    fn write_u64(&mut self, value: u64) {
        let mixed = (self.0 ^ value).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.0 = mixed ^ (mixed >> 29);
    }
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    slot: u32,
    /// The epoch the key was last drawn in.
    seen: u32,
}

/// The CPU side of the store: which slot each instance has, and a copy of what the GPU buffer holds (as bits, so
/// a `-0.0` replacing a `0.0` is still written).
#[derive(Default)]
pub(in crate::gpu) struct SlotTable {
    keys: HashMap<InstanceKey, Entry, BuildHasherDefault<KeyHasher>>,
    data: Vec<[u32; 20]>,
    free: Vec<u32>,
    /// The transient slots handed out this epoch, in order.
    transient: Vec<u32>,
    /// Last epoch's transient slots, handed out again in the same order; `reuse_at` is the next one.
    reuse: Vec<u32>,
    reuse_at: usize,
    /// Slots written since the last flush (unsorted, may repeat).
    dirty: Vec<u32>,
    epoch: u32,
}

impl SlotTable {
    /// Slots in use or free: the store's size.
    pub(in crate::gpu) fn len(&self) -> usize {
        self.data.len()
    }

    /// What every slot holds.
    pub(in crate::gpu) fn data(&self) -> &[[u32; 20]] {
        &self.data
    }

    /// Start an epoch: last epoch's unused transient slots and the keys not drawn for [`RETAIN_EPOCHS`] epochs
    /// give their slots back. Only call it when nothing recorded with the store is still waiting to be submitted.
    pub(in crate::gpu) fn begin_epoch(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        let unused = self.reuse_at.min(self.reuse.len());
        self.free.extend(self.reuse.drain(unused..));
        self.reuse = std::mem::take(&mut self.transient);
        self.reuse_at = 0;
        let epoch = self.epoch;
        let free = &mut self.free;
        self.keys.retain(|_, entry| {
            let keep = epoch.wrapping_sub(entry.seen) <= RETAIN_EPOCHS;
            if !keep {
                free.push(entry.slot);
            }
            keep
        });
    }

    /// The slot of an instance with `key` and model matrix `data` for this epoch, writing the matrix if the slot
    /// held something else.
    pub(in crate::gpu) fn assign(&mut self, key: InstanceKey, data: [u32; 20]) -> u32 {
        if key.is_transient() {
            return self.claim_transient(data);
        }
        let epoch = self.epoch;
        let found = self.keys.get_mut(&key).map(|entry| {
            let before = entry.seen;
            entry.seen = epoch;
            (entry.slot, before)
        });
        let Some((slot, seen)) = found else {
            let slot = self.allocate();
            self.keys.insert(key, Entry { slot, seen: epoch });
            self.write(slot, data);
            return slot;
        };
        // Another view of this epoch drew the key elsewhere: keep its matrix for that view.
        if seen == epoch && self.data[slot as usize] != data {
            return self.claim_transient(data);
        }
        self.write(slot, data);
        slot
    }

    fn claim_transient(&mut self, data: [u32; 20]) -> u32 {
        let slot = match self.reuse.get(self.reuse_at) {
            Some(&slot) => {
                self.reuse_at += 1;
                slot
            }
            None => self.allocate(),
        };
        self.transient.push(slot);
        self.write(slot, data);
        slot
    }

    fn allocate(&mut self) -> u32 {
        if let Some(slot) = self.free.pop() {
            return slot;
        }
        // A new slot holds zeros, like the zero-initialised GPU buffer.
        self.data.push([0; 20]);
        (self.data.len() - 1) as u32
    }

    fn write(&mut self, slot: u32, data: [u32; 20]) {
        let stored = &mut self.data[slot as usize];
        if *stored == data {
            return;
        }
        *stored = data;
        self.dirty.push(slot);
    }

    /// The slots written since the last call, as few ranges as [`JOIN_GAP`] allows, into `out`.
    pub(in crate::gpu) fn take_dirty(&mut self, out: &mut Vec<Range<u32>>) {
        coalesce(&mut self.dirty, JOIN_GAP, out);
    }

    /// Forget the written slots (the whole store is being uploaded).
    fn clear_dirty(&mut self) {
        self.dirty.clear();
    }
}

/// Turn the slots in `dirty` into sorted ranges in `out`, joining two ranges when at most `gap` slots lie between
/// them. `dirty` is left empty.
pub(in crate::gpu) fn coalesce(dirty: &mut Vec<u32>, gap: u32, out: &mut Vec<Range<u32>>) {
    out.clear();
    dirty.sort_unstable();
    dirty.dedup();
    for &slot in dirty.iter() {
        match out.last_mut() {
            Some(last) if slot <= last.end.saturating_add(gap) => last.end = slot + 1,
            _ => out.push(slot..slot + 1),
        }
    }
    dirty.clear();
}

/// The store's GPU buffer and the bind group the pipelines read it through.
struct Gpu {
    buffer: wgpu::Buffer,
    /// Capacity in slots.
    capacity: u32,
    /// The most slots one binding may hold on this device.
    max: u32,
    layout: wgpu::BindGroupLayout,
    group: wgpu::BindGroup,
}

impl Gpu {
    fn new(device: &wgpu::Device, layout: wgpu::BindGroupLayout, capacity: u32, max: u32) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance store"),
            size: u64::from(capacity) * SLOT_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("instance store"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        Self { buffer, capacity, max, layout, group }
    }
}

/// The store, kept on the renderer. Created lazily on the first upload, since whether the device can use it is
/// only known then.
#[derive(Default)]
pub(in crate::gpu) struct InstanceStore {
    table: SlotTable,
    gpu: Option<Gpu>,
    enabled: Option<bool>,
    ranges: Vec<Range<u32>>,
    /// The slots of the last view assigned, in instance order.
    pub(in crate::gpu) slots: Vec<u32>,
    /// The slots of the main view's instances, for the passes that draw them again (shadows).
    main_slots: Vec<u32>,
    /// Where the main view's slot list starts in the renderer's instance buffer.
    main_offset: u64,
    /// The main view's instances were uploaded by this frame's scene and not yet drawn by its colour pass.
    main_ready: bool,
    /// Scratch for the matrices a view still uploads itself (glossy draws).
    pub(in crate::gpu) matrices: Vec<[f32; 20]>,
    overflow_logged: bool,
}

impl InstanceStore {
    /// Whether this device uses the store (asked once).
    pub(in crate::gpu) fn enabled(&mut self, device: &wgpu::Device) -> bool {
        *self.enabled.get_or_insert_with(|| supported(device))
    }

    /// The store's bind group, once it exists. `None` when the device does not use the store.
    pub(in crate::gpu) fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.gpu.as_ref().map(|gpu| &gpu.group)
    }

    /// Start a frame's epoch. Every encoder recorded with the store before this must have been submitted.
    pub(in crate::gpu) fn begin_frame(&mut self) {
        self.table.begin_epoch();
        self.main_ready = false;
    }

    /// The slots of `instances` for this epoch, into `out`, in order.
    pub(in crate::gpu) fn assign(&mut self, instances: &[Instance], out: &mut Vec<u32>) {
        out.clear();
        let table = &mut self.table;
        out.extend(
            instances.iter().map(|i| table.assign(i.key, bytemuck::cast(crate::gpu::instances::instance_words(i)))),
        );
    }

    /// Write the changed slots to the GPU buffer, growing it (and rewriting it whole) when it is too small.
    pub(in crate::gpu) fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let needed = self.table.len() as u32;
        let gpu = self.gpu.get_or_insert_with(|| {
            let max = (device.limits().max_storage_buffer_binding_size / SLOT_BYTES).min(u64::from(u32::MAX)) as u32;
            Gpu::new(device, layout(device), INITIAL_SLOTS.min(max), max)
        });
        if needed > gpu.capacity && gpu.capacity < gpu.max {
            let capacity = needed.next_power_of_two().min(gpu.max);
            *gpu = Gpu::new(device, gpu.layout.clone(), capacity, gpu.max);
            let whole = needed.min(capacity) as usize;
            queue.write_buffer(&gpu.buffer, 0, bytemuck::cast_slice(&self.table.data()[..whole]));
            self.table.clear_dirty();
            return;
        }
        if needed > gpu.capacity && !self.overflow_logged {
            log::error!("instance store is full at {} slots; later instances draw stale matrices", gpu.capacity);
            self.overflow_logged = true;
        }
        self.table.take_dirty(&mut self.ranges);
        let capacity = gpu.capacity;
        for range in &self.ranges {
            let end = range.end.min(capacity);
            if range.start >= end {
                continue;
            }
            let data = &self.table.data()[range.start as usize..end as usize];
            queue.write_buffer(&gpu.buffer, u64::from(range.start) * SLOT_BYTES, bytemuck::cast_slice(data));
        }
    }

    /// Record that the last view assigned (in [`Self::slots`]) is the main view, its slot list at `offset`.
    pub(in crate::gpu) fn finish_main(&mut self, offset: u64) {
        std::mem::swap(&mut self.slots, &mut self.main_slots);
        self.main_offset = offset;
        self.main_ready = true;
    }

    /// Whether the main view was uploaded and not yet drawn; drawing it clears that.
    pub(in crate::gpu) fn take_main_ready(&mut self) -> bool {
        std::mem::replace(&mut self.main_ready, false)
    }

    /// Where the main view's slot list starts in the renderer's instance buffer (0 without the store).
    pub(in crate::gpu) fn main_offset(&self) -> u64 {
        self.main_offset
    }

    /// The main view's slots, in instance order.
    pub(in crate::gpu) fn main_slots(&self) -> &[u32] {
        &self.main_slots
    }
}

#[cfg(test)]
#[path = "instance_store_tests.rs"]
mod tests;
