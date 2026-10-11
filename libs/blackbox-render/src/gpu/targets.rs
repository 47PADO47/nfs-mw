//! What the scene is drawn into: straight into the output image, or into an offscreen colour image
//! (HDR or the surface's own format) and a depth buffer, both at the internal render size.

use super::resources::create_depth;

pub(super) use blackbox_gpu_passes::{HDR_FORMAT, write_mask};

/// The format an HDR scene is drawn in: [`HDR_FORMAT`] when the adapter can render to it, blend into
/// it and sample it with a filtering sampler, otherwise the surface's own `fallback` format (the
/// frame then looks exactly as it did before the HDR target existed).
pub(super) fn pick_color_format(adapter: &wgpu::Adapter, fallback: wgpu::TextureFormat) -> wgpu::TextureFormat {
    let features = adapter.get_texture_format_features(HDR_FORMAT);
    let usages = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    let flags = wgpu::TextureFormatFeatureFlags::FILTERABLE | wgpu::TextureFormatFeatureFlags::BLENDABLE;
    if features.allowed_usages.contains(usages) && features.flags.contains(flags) {
        return HDR_FORMAT;
    }
    log::info!("{HDR_FORMAT:?} is not renderable here; effects that need HDR draw the scene in {fallback:?} instead");
    fallback
}

/// What the renderer knows when it decides where the scene goes.
pub(super) struct SceneInputs {
    /// The size of the image the frame ends up in (the surface, or a capture).
    pub output: (u32, u32),
    /// The size the scene is drawn at (`output` times the render scale).
    pub render: (u32, u32),
    /// Whether any pass runs besides the final resolve (effects, upscalers).
    pub passes: bool,
    /// Whether a pass needs HDR values (bloom, tone mapping).
    pub hdr: bool,
    pub surface_format: wgpu::TextureFormat,
    /// [`HDR_FORMAT`], or the surface format when the adapter cannot render HDR.
    pub hdr_format: wgpu::TextureFormat,
}

/// Where the scene is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ScenePlan {
    /// Straight into the output image: no offscreen colour image, no resolve copy.
    pub direct: bool,
    /// The format of the image the scene pipelines write.
    pub format: wgpu::TextureFormat,
    /// The size of the image the scene pipelines write.
    pub size: (u32, u32),
}

/// The cheapest correct target. The scene goes straight into the output when nothing needs the
/// offscreen image: no pass but the resolve copy, and a render size equal to the output size. Else it
/// goes offscreen, in HDR only when a pass needs it and in the surface's own format otherwise (a
/// quarter to half the memory and bandwidth). A surface that encodes sRGB on write always gets the
/// HDR image, so blending stays in linear space as it was before.
pub(super) fn plan_scene(i: &SceneInputs) -> ScenePlan {
    let direct = !i.passes && i.render == i.output && !i.surface_format.is_srgb();
    if direct {
        return ScenePlan { direct, format: i.surface_format, size: i.output };
    }
    let hdr = i.hdr || i.surface_format.is_srgb();
    ScenePlan { direct, format: if hdr { i.hdr_format } else { i.surface_format }, size: i.render }
}

/// The scene's colour image and depth buffer. Post-process passes read both.
pub(super) struct FrameTargets {
    _color: wgpu::Texture,
    pub color_view: wgpu::TextureView,
    pub color_format: wgpu::TextureFormat,
    /// Reverse-Z depth (1 near, 0 far). Also bindable as a sampled depth texture.
    pub depth_view: wgpu::TextureView,
    pub size: (u32, u32),
}

impl FrameTargets {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, size: (u32, u32)) -> Self {
        let size = (size.0.max(1), size.1.max(1));
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scene colour"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = create_depth(device, size.0, size.1);
        Self { _color: color, color_view, color_format: format, depth_view, size }
    }
}

/// The images a frame's scene is drawn into.
pub(super) enum SceneTargets {
    /// The scene goes straight into the output image; only a depth buffer is allocated.
    Direct { depth_view: wgpu::TextureView, format: wgpu::TextureFormat, size: (u32, u32) },
    /// The scene goes into an offscreen image that the post chain reads.
    Offscreen(FrameTargets),
}

impl SceneTargets {
    pub(super) fn new(device: &wgpu::Device, plan: &ScenePlan) -> Self {
        if !plan.direct {
            return Self::Offscreen(FrameTargets::new(device, plan.format, plan.size));
        }
        let size = (plan.size.0.max(1), plan.size.1.max(1));
        Self::Direct { depth_view: create_depth(device, size.0, size.1), format: plan.format, size }
    }

    /// Whether these images are what `plan` asks for.
    pub(super) fn matches(&self, plan: &ScenePlan) -> bool {
        self.plan() == ScenePlan { size: (plan.size.0.max(1), plan.size.1.max(1)), ..*plan }
    }

    pub(super) fn plan(&self) -> ScenePlan {
        ScenePlan { direct: self.offscreen().is_none(), format: self.color_format(), size: self.size() }
    }

    pub(super) fn size(&self) -> (u32, u32) {
        match self {
            Self::Direct { size, .. } => *size,
            Self::Offscreen(targets) => targets.size,
        }
    }

    /// The format the scene pipelines write.
    pub(super) fn color_format(&self) -> wgpu::TextureFormat {
        match self {
            Self::Direct { format, .. } => *format,
            Self::Offscreen(targets) => targets.color_format,
        }
    }

    pub(super) fn depth_view(&self) -> &wgpu::TextureView {
        match self {
            Self::Direct { depth_view, .. } => depth_view,
            Self::Offscreen(targets) => &targets.depth_view,
        }
    }

    /// The offscreen images, or `None` when the scene goes straight into the output.
    pub(super) fn offscreen(&self) -> Option<&FrameTargets> {
        match self {
            Self::Direct { .. } => None,
            Self::Offscreen(targets) => Some(targets),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BGRA: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
    const SRGB: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

    fn inputs() -> SceneInputs {
        SceneInputs {
            output: (1280, 720),
            render: (1280, 720),
            passes: false,
            hdr: false,
            surface_format: BGRA,
            hdr_format: HDR_FORMAT,
        }
    }

    #[test]
    fn the_defaults_draw_straight_into_the_output() {
        let plan = plan_scene(&inputs());
        assert_eq!(plan, ScenePlan { direct: true, format: BGRA, size: (1280, 720) });
    }

    #[test]
    fn a_pass_or_a_different_render_size_needs_the_offscreen_image() {
        let with_pass = plan_scene(&SceneInputs { passes: true, ..inputs() });
        assert_eq!(with_pass, ScenePlan { direct: false, format: BGRA, size: (1280, 720) });
        let scaled = plan_scene(&SceneInputs { render: (960, 540), ..inputs() });
        assert_eq!(scaled, ScenePlan { direct: false, format: BGRA, size: (960, 540) });
        let supersampled = plan_scene(&SceneInputs { render: (2560, 1440), ..inputs() });
        assert!(!supersampled.direct);
    }

    #[test]
    fn only_bloom_and_tone_mapping_ask_for_hdr() {
        let hdr = plan_scene(&SceneInputs { passes: true, hdr: true, ..inputs() });
        assert_eq!(hdr.format, HDR_FORMAT);
        let fallback = plan_scene(&SceneInputs { passes: true, hdr: true, hdr_format: BGRA, ..inputs() });
        assert_eq!(fallback.format, BGRA, "an adapter without HDR keeps the surface format");
        let fxaa_or_fsr = plan_scene(&SceneInputs { passes: true, ..inputs() });
        assert_eq!(fxaa_or_fsr.format, BGRA);
    }

    #[test]
    fn an_srgb_surface_never_takes_the_fast_path_or_the_8_bit_image() {
        let plan = plan_scene(&SceneInputs { surface_format: SRGB, ..inputs() });
        assert_eq!(plan, ScenePlan { direct: false, format: HDR_FORMAT, size: (1280, 720) });
    }
}
