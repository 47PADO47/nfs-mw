//! Resources shared by every draw: the globals uniform, bind group layouts and the sampler (from
//! `blackbox-gpu-passes`, which the effect passes are built against), plus the glossy layouts.

pub(super) use blackbox_gpu_passes::{DEPTH_FORMAT, Globals, WorldBindings, create_depth};

pub(super) struct Shared {
    pub bindings: WorldBindings,
    pub glossy: super::glossy::Layouts,
}

impl Shared {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        Self { bindings: WorldBindings::new(device), glossy: super::glossy::Layouts::new(device) }
    }
}
