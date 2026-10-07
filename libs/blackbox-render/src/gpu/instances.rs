//! The per-frame instance buffer: one model matrix per placed mesh copy.

/// Four `vec4` columns of the model matrix, at shader locations 4–7.
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4];

pub(super) const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &INSTANCE_ATTRIBUTES,
};

pub(super) struct InstanceBuffer {
    pub buffer: wgpu::Buffer,
    /// Capacity in matrices.
    capacity: usize,
}

impl InstanceBuffer {
    const INITIAL: usize = 4096;

    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self { buffer: Self::allocate(device, Self::INITIAL), capacity: Self::INITIAL }
    }

    fn allocate(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (capacity * 64) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Upload this frame's matrices, growing the buffer when needed.
    pub(super) fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, matrices: &[[f32; 16]]) {
        if matrices.len() > self.capacity {
            self.capacity = matrices.len().next_power_of_two();
            self.buffer = Self::allocate(device, self.capacity);
        }
        if !matrices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(matrices));
        }
    }
}
