//! The WGSL sources of the passes, and how a module is put together from them.
//!
//! Every pass file declares its own bindings from binding 1 on and is compiled behind
//! `shaders/common.wgsl`, which holds the shared definitions and the uniform block at binding 0.

use wgpu::TextureFormat;

/// The definitions shared by all passes.
pub const COMMON: &str = include_str!("shaders/common.wgsl");

/// The `override` constants of the shaders that the pipelines set.
pub const OVERRIDE_NAMES: [&str; 6] = [
    "INVERTED_DEPTH",
    "HDR_INPUT",
    "LOW_RES_MOTION_VECTORS",
    "JITTERED_MOTION_VECTORS",
    "AUTO_EXPOSURE",
    "APPLY_SHARPENING",
];

/// A pass of the upscaler, in the order they run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pass {
    /// Depth extents, dilated motion vectors, reconstructed previous depth and luma.
    PrepareInputs,
    /// The half-resolution farthest depth, the frame's average luminance and the exposure.
    LumaPyramid,
    /// The luma difference to the previous frame and its mean pyramid.
    ShadingChangePyramid,
    /// The per-pixel shading change estimate.
    ShadingChange,
    /// Disocclusion, reactiveness, accumulation and locks.
    PrepareReactivity,
    /// Flicker detection on the luma history.
    LumaInstability,
    /// The upsample, reprojection, rectification and blend.
    Accumulate,
    /// Contrast-adaptive sharpening.
    Rcas,
}

impl Pass {
    /// Every pass, in the order they run.
    pub const ALL: [Self; 8] = [
        Self::PrepareInputs,
        Self::LumaPyramid,
        Self::ShadingChangePyramid,
        Self::ShadingChange,
        Self::PrepareReactivity,
        Self::LumaInstability,
        Self::Accumulate,
        Self::Rcas,
    ];

    /// The file name of the pass under `src/shaders`.
    pub fn file_name(self) -> &'static str {
        match self {
            Self::PrepareInputs => "prepare_inputs.wgsl",
            Self::LumaPyramid => "luma_pyramid.wgsl",
            Self::ShadingChangePyramid => "shading_change_pyramid.wgsl",
            Self::ShadingChange => "shading_change.wgsl",
            Self::PrepareReactivity => "prepare_reactivity.wgsl",
            Self::LumaInstability => "luma_instability.wgsl",
            Self::Accumulate => "accumulate.wgsl",
            Self::Rcas => "rcas.wgsl",
        }
    }

    /// The pass's own source, without the shared definitions.
    pub fn source(self) -> &'static str {
        match self {
            Self::PrepareInputs => include_str!("shaders/prepare_inputs.wgsl"),
            Self::LumaPyramid => include_str!("shaders/luma_pyramid.wgsl"),
            Self::ShadingChangePyramid => include_str!("shaders/shading_change_pyramid.wgsl"),
            Self::ShadingChange => include_str!("shaders/shading_change.wgsl"),
            Self::PrepareReactivity => include_str!("shaders/prepare_reactivity.wgsl"),
            Self::LumaInstability => include_str!("shaders/luma_instability.wgsl"),
            Self::Accumulate => include_str!("shaders/accumulate.wgsl"),
            Self::Rcas => include_str!("shaders/rcas.wgsl"),
        }
    }

    /// The compute entry points of the pass.
    pub fn entry_points(self) -> &'static [&'static str] {
        match self {
            Self::LumaPyramid => &["cs_reduce", "cs_final"],
            Self::ShadingChangePyramid => &["cs_level0", "cs_down"],
            _ => &["cs_main"],
        }
    }
}

/// The WGSL name of a storage texture format, as written in a `texture_storage_2d`.
pub fn storage_format_name(format: TextureFormat) -> Option<&'static str> {
    match format {
        TextureFormat::Rgba16Float => Some("rgba16float"),
        TextureFormat::Rgba32Float => Some("rgba32float"),
        TextureFormat::Rgba8Unorm => Some("rgba8unorm"),
        _ => None,
    }
}

/// The module source of a pass: the shared definitions, then the pass, with the output texture's
/// storage format filled in.
pub fn module_source(pass: Pass, output_format: TextureFormat) -> String {
    let format = storage_format_name(output_format).unwrap_or("rgba16float");
    let mut source = String::with_capacity(COMMON.len() + pass.source().len() + 1);
    source.push_str(COMMON);
    source.push('\n');
    source.push_str(&pass.source().replace("OUTPUT_FORMAT", format));
    source
}
