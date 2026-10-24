//! The per-frame instance buffer: one unchanged model matrix and an explicit receiver flag per placed mesh copy.

/// Four `vec4` columns of the model matrix, at shader locations 4–7.
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 5] =
    wgpu::vertex_attr_array![4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 12 => Float32x4];

pub(super) const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 80,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &INSTANCE_ATTRIBUTES,
};

pub(super) struct InstanceBuffer {
    pub buffer: wgpu::Buffer,
    /// Capacity in matrices.
    capacity: usize,
    scratch: Vec<[f32; 20]>,
}

impl InstanceBuffer {
    const INITIAL: usize = 4096;

    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self { buffer: Self::allocate(device, Self::INITIAL), capacity: Self::INITIAL, scratch: Vec::new() }
    }

    fn allocate(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (capacity * 80) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Grow the buffer to hold `len` matrices. True when it was replaced, so nothing written before is in it.
    pub(super) fn reserve(&mut self, device: &wgpu::Device, len: usize) -> bool {
        if len <= self.capacity {
            return false;
        }
        self.capacity = len.next_power_of_two();
        self.buffer = Self::allocate(device, self.capacity);
        true
    }

    /// Overwrite the matrices from index `first` on, after a [`reserve`](Self::reserve) that made room.
    pub(super) fn write_at<T: bytemuck::Pod>(&self, queue: &wgpu::Queue, first: usize, matrices: &[T]) {
        if matrices.is_empty() {
            return;
        }
        let offset = (first * std::mem::size_of::<T>()) as u64;
        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(matrices));
    }

    /// Upload this frame's matrices, growing the buffer when needed.
    pub(super) fn upload<T: bytemuck::Pod>(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, matrices: &[T]) {
        self.reserve(device, matrices.len());
        if !matrices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(matrices));
        }
    }

    pub(super) fn upload_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[blackbox_gfx::Instance],
    ) {
        let mut matrices = std::mem::take(&mut self.scratch);
        matrices.clear();
        matrices.extend(instances.iter().map(instance_words));
        self.upload(device, queue, &matrices);
        self.scratch = matrices;
    }
}

/// Explicit per-instance receiver metadata follows the unchanged model matrix. Padding matches WGSL's vec4 alignment.
pub(in crate::gpu) fn instance_words(instance: &blackbox_gfx::Instance) -> [f32; 20] {
    let mut words = [0.0; 20];
    words[..16].copy_from_slice(&instance.transform.to_cols_array());
    words[16] = f32::from(u8::from(instance.shadow.receive));
    words
}
