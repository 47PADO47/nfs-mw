//! Resources shared by every draw: the globals uniform, bind group layouts and the sampler (from
//! `blackbox-gpu-passes`, which the effect passes are built against), plus the glossy layouts.

pub(super) use blackbox_gpu_passes::{DEPTH_FORMAT, Globals, WorldBindings};

pub(super) fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    #[cfg(all(windows, feature = "streamline"))]
    if let Some(view) = super::dlss::dx12::depth(device, width, height) {
        return view;
    }
    blackbox_gpu_passes::create_depth(device, width, height)
}

pub(super) struct Shared {
    pub bindings: WorldBindings,
    pub glossy: super::glossy::Layouts,
    pub shadows: super::shadows::maps::Maps,
}

impl Shared {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let glossy = super::glossy::Layouts::new(device);
        let shadows = super::shadows::maps::Maps::new(device, &glossy.material, 1, 4, None, None, None);
        Self { bindings: WorldBindings::new(device), glossy, shadows }
    }
}
