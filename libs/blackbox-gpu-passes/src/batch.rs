//! A vertex buffer for one batch of effect vertices that grows when needed and is reused every frame.

use blackbox_gfx::EffectVertex;

/// One batch of [`EffectVertex`]es on the GPU. The buffer only grows (to the next power of two), so a
/// batch that is emptied and refilled does not allocate again.
pub struct Batch {
    buffer: wgpu::Buffer,
    capacity: usize,
    count: u32,
}

impl Batch {
    pub fn new(device: &wgpu::Device) -> Self {
        Self { buffer: Self::allocate(device, 1), capacity: 1, count: 0 }
    }

    fn allocate(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamic effects"),
            size: (capacity * std::mem::size_of::<EffectVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Replace the batch's vertices. An empty slice empties the batch and keeps the allocation.
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[EffectVertex]) {
        self.count = vertices.len() as u32;
        if vertices.len() > self.capacity {
            self.capacity = vertices.len().next_power_of_two();
            self.buffer = Self::allocate(device, self.capacity);
        }
        if !vertices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    /// The vertex buffer; draw `0..self.count()` from it.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// The vertices currently in the batch.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// The allocated size in vertices.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
