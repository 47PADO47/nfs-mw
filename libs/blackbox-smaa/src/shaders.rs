//! The WGSL of the three passes. Each pass is one module: the shared header (`common.wgsl`) followed by the pass's
//! own file. Every file stays under 500 lines, and the sources are compiled into the crate, so no file is read at
//! run time.

/// Pass 1: luma edge detection (`vs_main`, `fs_main`).
pub(crate) const EDGE_DETECT_WGSL: &str =
    concat!(include_str!("shaders/common.wgsl"), include_str!("shaders/edge_detect.wgsl"));

/// Pass 2: blending weights (`vs_main`, `fs_main`), with the search helpers between the header and the pass.
pub(crate) const BLEND_WEIGHT_WGSL: &str = concat!(
    include_str!("shaders/common.wgsl"),
    include_str!("shaders/blend_search.wgsl"),
    include_str!("shaders/blend_weight.wgsl"),
);

/// Pass 3: neighbourhood blending (`vs_main`, `fs_main`).
pub(crate) const NEIGHBORHOOD_WGSL: &str =
    concat!(include_str!("shaders/common.wgsl"), include_str!("shaders/neighborhood.wgsl"));
