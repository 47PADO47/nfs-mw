//! One view's instance upload and the instance input of the shaders that read it.
//!
//! With the instance store a view's buffer holds two regions: from 0, one matrix per instance position, written
//! only for runs of meshes with glossy draws (the glossy pipelines read their matrices as vertex attributes); then
//! the slot list, one `u32` per instance, which every other pipeline reads as its per-instance attribute. Without
//! the store the buffer holds every matrix from 0, as before.

use super::instance_store::InstanceStore;
use crate::gpu::{
    Renderer,
    instances::{INSTANCE_LAYOUT, InstanceBuffer},
    meshes::GpuMesh,
    slots::Slots,
};
use blackbox_gfx::{Instance, Shading};

const SLOT_ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![4 => Uint32];

/// The per-instance slot index, at shader location 4.
const SLOT_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 4,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &SLOT_ATTRIBUTES,
};

/// The per-instance vertex buffer layout of a pipeline that reads the store (`store`) or the matrices.
pub(in crate::gpu) fn instance_layout(store: bool) -> wgpu::VertexBufferLayout<'static> {
    match store {
        true => SLOT_LAYOUT,
        false => INSTANCE_LAYOUT,
    }
}

const BEGIN: &str = "// instance input: begin";
const END: &str = "// instance input: end";

/// `source` with its instance input block replaced by the store's when `group` is the bind group the store is
/// read through. Each scene shader declares `InstanceIn` and `instance_model` between the two markers, reading
/// the matrix from vertex attributes 4–7; the store's version reads a slot at location 4 and loads the matrix.
pub(in crate::gpu) fn instance_source(source: &str, group: Option<u32>) -> String {
    let Some(group) = group else { return source.to_owned() };
    let (Some(start), Some(end)) = (source.find(BEGIN), source.find(END)) else {
        panic!("the shader has no instance input block");
    };
    format!(
        "{}struct InstanceIn {{ @location(4) slot: u32, }};\n\
         @group({group}) @binding(0) var<storage, read> instance_models: array<mat4x4<f32>>;\n\
         fn instance_model(i: InstanceIn) -> mat4x4<f32> {{ return instance_models[i.slot]; }}\n{}",
        &source[..start],
        &source[end + END.len()..]
    )
}

/// Upload one view's `instances` into `buffer`: through the store when the device has it, else as matrices.
/// Returns where the view's per-instance data for the non-glossy pipelines starts in `buffer`.
pub(in crate::gpu) fn upload_view(
    store: &mut InstanceStore,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    meshes: &Slots<GpuMesh>,
    buffer: &mut InstanceBuffer,
    instances: &[Instance],
) -> u64 {
    if !store.enabled(device) {
        buffer.upload_instances(device, queue, instances);
        return 0;
    }
    let _trace = blackbox_trace::scope("Renderer CPU: instance slots");
    let mut slots = std::mem::take(&mut store.slots);
    store.assign(instances, &mut slots);
    store.flush(device, queue);
    let count = instances.len() as u64;
    let region = count * 64;
    reserve(buffer, device, queue, region + count * 4);
    let mut matrices = std::mem::take(&mut store.matrices);
    write_glossy_runs(queue, &buffer.buffer, meshes, instances, &mut matrices);
    store.matrices = matrices;
    if !slots.is_empty() {
        queue.write_buffer(&buffer.buffer, region, bytemuck::cast_slice(&slots));
    }
    store.slots = slots;
    region
}

/// Write `words` at the start of `buffer`, growing it when needed.
pub(in crate::gpu) fn upload_words(
    buffer: &mut InstanceBuffer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    words: &[u32],
) {
    reserve(buffer, device, queue, words.len() as u64 * 4);
    if words.is_empty() {
        return;
    }
    queue.write_buffer(&buffer.buffer, 0, bytemuck::cast_slice(words));
}

/// Grow `buffer` to at least `bytes`. Rare: the buffer doubles, and is zeroed once when it does.
fn reserve(buffer: &mut InstanceBuffer, device: &wgpu::Device, queue: &wgpu::Queue, bytes: u64) {
    if buffer.buffer.size() >= bytes {
        return;
    }
    let entries = bytes.div_ceil(64) as usize;
    buffer.upload(device, queue, &vec![[0.0_f32; 16]; entries]);
}

/// Write the matrices of every run of instances whose mesh has a glossy draw at their positions in `buffer`,
/// adjacent runs in one write.
fn write_glossy_runs(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    meshes: &Slots<GpuMesh>,
    instances: &[Instance],
    matrices: &mut Vec<[f32; 16]>,
) {
    matrices.clear();
    let mut pending = 0;
    let mut start = 0;
    while start < instances.len() {
        let mesh = instances[start].mesh;
        let end = start + instances[start..].iter().take_while(|i| i.mesh == mesh).count();
        let glossy =
            meshes.get(mesh.raw()).is_some_and(|m| m.draws.iter().any(|d| matches!(d.shading, Shading::Glossy(_))));
        if glossy && matrices.is_empty() {
            pending = start;
        }
        if glossy {
            matrices.extend(instances[start..end].iter().map(|i| i.transform.to_cols_array()));
        }
        if !glossy && !matrices.is_empty() {
            queue.write_buffer(buffer, pending as u64 * 64, bytemuck::cast_slice(matrices.as_slice()));
            matrices.clear();
        }
        start = end;
    }
    if !matrices.is_empty() {
        queue.write_buffer(buffer, pending as u64 * 64, bytemuck::cast_slice(matrices.as_slice()));
        matrices.clear();
    }
}

impl Renderer {
    /// Upload the main view's instances into the renderer's instance buffer, for its colour pass and the passes
    /// that draw the same instances (prepass, motion vectors, shadows).
    pub(in crate::gpu) fn upload_main_instances(&mut self, instances: &[Instance]) {
        let store = &mut self.draw_list.store;
        let offset = upload_view(store, &self.device, &self.queue, &self.meshes, &mut self.instances, instances);
        store.finish_main(offset);
    }

    /// Upload the main view's instances unless this frame's scene already did.
    pub(in crate::gpu) fn ensure_main_instances(&mut self, instances: &[Instance]) {
        if self.draw_list.store.take_main_ready() {
            return;
        }
        self.upload_main_instances(instances);
        self.draw_list.store.take_main_ready();
    }
}
