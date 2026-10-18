//! Explicit HDR presentation, independently of artistic scene grading.
use super::{Renderer, output::Output, post::display::Mapping};
use blackbox_gfx::{DisplaySettings, DisplayStatus, HdrMode, display::hdr_active};
use std::time::{Duration, Instant};

pub(super) struct Display {
    settings: DisplaySettings,
    pub status: DisplayStatus,
    sdr_format: wgpu::TextureFormat,
    supported: bool,
    refreshed: Option<Instant>,
}
impl Display {
    pub(super) fn new(output: &Output, adapter: &wgpu::Adapter) -> Self {
        let supported = match output {
            Output::Window { surface, .. } => surface
                .get_capabilities(adapter)
                .color_spaces(wgpu::TextureFormat::Rgba16Float)
                .contains(wgpu::SurfaceColorSpaces::EXTENDED_SRGB_LINEAR),
            Output::Texture { .. } => false,
        };
        Self {
            settings: Default::default(),
            status: Default::default(),
            sdr_format: output.format(),
            supported,
            refreshed: None,
        }
    }
}
impl Renderer {
    pub fn set_display_settings(&mut self, settings: DisplaySettings) {
        let settings = settings.sanitized();
        if settings == self.display.settings && self.display.refreshed.is_some() {
            return;
        }
        self.display.settings = settings;
        self.refresh_display(true);
    }
    pub fn display_status(&self) -> DisplayStatus {
        self.display.status
    }

    pub(super) fn refresh_display(&mut self, force: bool) {
        if !force && self.display.refreshed.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
            return;
        }
        self.display.refreshed = Some(Instant::now());
        let Output::Window { surface, config } = &mut self.output else {
            return;
        };
        let capabilities = surface.get_capabilities(&self.adapter);
        #[cfg(all(windows, feature = "streamline"))]
        let generation_requested = self.dlss.is_some()
            && self.presentation.status.max_generated_frames > 0
            && self.presentation.request.generated_frames > 0;
        #[cfg(not(all(windows, feature = "streamline")))]
        let generation_requested = false;
        let pq_supported =
            capabilities.color_spaces(wgpu::TextureFormat::Rgb10a2Unorm).contains(wgpu::SurfaceColorSpaces::BT2100_PQ);
        let metadata = surface.display_hdr_info(&self.adapter);
        self.display.supported = surface
            .get_capabilities(&self.adapter)
            .color_spaces(wgpu::TextureFormat::Rgba16Float)
            .contains(wgpu::SurfaceColorSpaces::EXTENDED_SRGB_LINEAR);
        let os_hdr = metadata.coarse.and_then(|c| c.high_dynamic_range);
        let reported =
            metadata.luminance.and_then(|l| l.max_nits).filter(|v| v.is_finite() && (80.0..=10000.0).contains(v));
        let s = self.display.settings;
        let supported = self.display.supported && self.hdr_format == wgpu::TextureFormat::Rgba16Float;
        let active = hdr_active(s.hdr, supported, os_hdr);
        let peak = s.peak(reported);
        let pq = active && generation_requested && pq_supported;
        let note = match (active, s.hdr, supported, os_hdr, reported) {
            (true, _, _, _, _) if pq => "HDR10 BT.2100 PQ / PsychoV; separate scene and UI recomposition.",
            (true, _, _, _, _) if generation_requested => {
                "HDR scRGB; this surface does not offer HDR10 required for frame generation."
            }
            (true, _, _, _, None) if s.peak_nits == 0.0 => {
                "HDR scRGB / PsychoV; peak fallback 1000 nits. Calibrate for your display."
            }
            (true, _, _, _, _) => "HDR scRGB / PsychoV. Display peak is advisory; manual calibration is recommended.",
            (false, HdrMode::Off, _, _, _) => "SDR output selected.",
            (false, _, false, _, _) => "This surface has no RGBA16F scRGB output support; using SDR.",
            (false, _, _, Some(false), _) => "The OS reports SDR. Enable HDR for this display to use HDR output.",
            _ => "HDR display state is unknown. Auto retains SDR; On can explicitly opt in.",
        };
        let status = DisplayStatus {
            supported,
            active,
            reported_peak_nits: reported,
            peak_nits: peak,
            scene_white_nits: s.scene_white_nits.min(peak),
            ui_white_nits: s.ui_white_nits.min(peak),
            note,
        };
        let format = match (active, pq) {
            (_, true) => wgpu::TextureFormat::Rgb10a2Unorm,
            (true, false) => wgpu::TextureFormat::Rgba16Float,
            _ => self.display.sdr_format,
        };
        let color_space = match (active, pq) {
            (_, true) => wgpu::SurfaceColorSpace::Bt2100Pq,
            (true, false) => wgpu::SurfaceColorSpace::ExtendedSrgbLinear,
            _ => wgpu::SurfaceColorSpace::Auto,
        };
        if status == self.display.status && config.format == format && config.color_space == color_space {
            return;
        }
        if config.format != format || config.color_space != color_space {
            config.format = format;
            config.color_space = color_space;
            #[cfg(all(windows, feature = "streamline"))]
            if self.dlss.is_some() {
                self.presentation.suspend(&self.device);
            }
            surface.configure(&self.device, config);
        }
        self.display.status = status;
        self.post.set_display(
            &self.device,
            active.then_some(Mapping {
                peak: peak / status.scene_white_nits,
                white: status.scene_white_nits,
                linear: true,
            }),
        );
        self.ui.set_hdr_white(active.then_some(status.ui_white_nits));
        self.refresh_targets();
        log::info!(
            "display: {note} effective peak={peak:.0}, scene white={:.0}, UI white={:.0} nits; OS HDR={os_hdr:?}",
            status.scene_white_nits,
            status.ui_white_nits
        );
    }
}
