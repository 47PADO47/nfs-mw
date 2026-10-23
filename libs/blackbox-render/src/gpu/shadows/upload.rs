//! The caster upload of a cascade: the casters' slots in the instance store, or their matrices without the store.

use crate::gpu::draw_list::{instance_store::InstanceStore, instance_view::upload_words};
use crate::gpu::instances::InstanceBuffer;
use blackbox_gfx::Instance;

/// Reused space for one cascade's upload.
#[derive(Default)]
pub(super) struct CasterScratch {
    words: Vec<u32>,
    matrices: Vec<[f32; 16]>,
}

impl CasterScratch {
    /// Upload the casters `picked` (indices into `casters`), in order, into `buffer`: their slots from `slots`
    /// (one per caster) when the casters are read from the instance store, else their matrices.
    pub(super) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &mut InstanceBuffer,
        casters: &[Instance],
        picked: &[usize],
        slots: Option<&[u32]>,
    ) {
        let Some(slots) = slots else {
            self.matrices.clear();
            self.matrices.extend(picked.iter().map(|&i| casters[i].transform.to_cols_array()));
            buffer.upload(device, queue, &self.matrices);
            return;
        };
        self.words.clear();
        self.words.extend(picked.iter().map(|&i| slots[i]));
        upload_words(buffer, device, queue, &self.words);
    }
}

/// The store slots of `casters`, one per caster, into `out`. `main_view` says the casters are the main view's
/// instances, which this frame's scene already gave slots; other casters are given slots here.
pub(super) fn caster_slots(
    store: &mut InstanceStore,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    casters: &[Instance],
    main_view: bool,
    out: &mut Vec<u32>,
) {
    out.clear();
    if main_view && store.main_slots().len() == casters.len() {
        out.extend_from_slice(store.main_slots());
        return;
    }
    store.assign(casters, out);
    store.flush(device, queue);
}
