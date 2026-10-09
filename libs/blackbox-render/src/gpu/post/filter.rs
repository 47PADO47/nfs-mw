//! The fullscreen filter helper (from `blackbox-gpu-passes`) and the shader modules of the effects built
//! on it: the shared bindings and vertex stage, then the effect's own fragment stages. Functions so the
//! tests can validate them without a GPU.

pub(super) use blackbox_gpu_passes::{Blend, Draw, Filter, Params, filter_source};

pub(super) fn tonemap_wgsl() -> String {
    filter_source(include_str!("../../shaders/tonemap.wgsl"))
}

pub(super) fn bloom_wgsl() -> String {
    filter_source(include_str!("../../shaders/bloom.wgsl"))
}

pub(super) fn fxaa_wgsl() -> String {
    filter_source(include_str!("../../shaders/fxaa.wgsl"))
}
