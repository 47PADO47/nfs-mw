//! The real DLSS Super Resolution path (`dlss`, not `dlss-mock`): the project id and `DlssInitPlugin` the app adds
//! before `RenderPlugin`, and Bevy's `Dlss` camera component.
//!
//! Written against `bevy_anti_alias` 0.20.0's `dlss` module (its source in the cargo registry) and `dlss_wgpu` 6.0.0.
//! It needs the DLSS SDK to build, so it has not been compiled or run in the development environment. Keep it small.

use bevy_anti_alias::dlss::{Dlss, DlssPerfQualityMode, DlssProjectId, DlssSuperResolutionFeature};
use bevy_camera::Hdr;
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Commands;
use bevy_render::camera::{MipBias, TemporalJitter};
use blackbox_gfx::{Antialiasing, UpscaleQuality, Upscaler};

use crate::ops::CameraSettings;

/// This app's DLSS project id (a random UUID, generated for this project). NVIDIA issues ids per application; this
/// one is the project's own and is not a secret.
pub const DLSS_PROJECT_ID: &str = "012a1384-4277-448b-87fa-88b528a7ad69";

/// The id `DlssInitPlugin` reads. It is inserted before the plugin is added.
pub fn project_id() -> DlssProjectId {
    DlssProjectId(DLSS_PROJECT_ID.parse().expect("DLSS_PROJECT_ID is a UUID"))
}

/// The DLSS mode for a quality mode. `Auto` is `Quality`, the mode `UpscaleQuality::Auto` already renders at.
pub fn perf_mode(quality: UpscaleQuality) -> DlssPerfQualityMode {
    match quality {
        UpscaleQuality::Native => DlssPerfQualityMode::Dlaa,
        UpscaleQuality::Auto | UpscaleQuality::Quality => DlssPerfQualityMode::Quality,
        UpscaleQuality::Balanced => DlssPerfQualityMode::Balanced,
        UpscaleQuality::Performance => DlssPerfQualityMode::Performance,
        UpscaleQuality::UltraPerformance => DlssPerfQualityMode::UltraPerformance,
    }
}

/// The quality mode whose render scale is nearest `scale`. The settings carry the scale, not the mode, and the
/// scale came from the table, so the nearest entry is the mode it came from.
pub fn quality_for_scale(scale: f32) -> UpscaleQuality {
    const MODES: [UpscaleQuality; 5] = [
        UpscaleQuality::Native,
        UpscaleQuality::Quality,
        UpscaleQuality::Balanced,
        UpscaleQuality::Performance,
        UpscaleQuality::UltraPerformance,
    ];
    let distance = |quality: &UpscaleQuality| (quality.render_scale() - scale).abs();
    MODES.into_iter().min_by(|a, b| distance(a).total_cmp(&distance(b))).unwrap_or(UpscaleQuality::Quality)
}

/// Put the DLSS camera state on `camera`: the `Dlss` component while DLSS is the upscaler, removed otherwise.
///
/// Runs every frame, after `post::set` (`apply/mod.rs`). `post::taa::set` removes the prepasses and the jitter when
/// TAA is off, and `Dlss` needs them, so they are inserted again here, after it.
pub fn set(commands: &mut Commands, camera: Entity, settings: &CameraSettings, camera_cut: bool) {
    let mut entity = commands.entity(camera);
    if settings.upscaler != Upscaler::Dlss {
        entity.remove::<Dlss>();
        return;
    }
    let mut dlss = Dlss::<DlssSuperResolutionFeature>::default();
    dlss.perf_quality_mode = perf_mode(quality_for_scale(settings.render_scale));
    dlss.reset = camera_cut;
    entity.insert((
        dlss,
        DepthPrepass,
        MotionVectorPrepass,
        TemporalJitter::default(),
        Hdr,
        // Bevy's `prepare_dlss` sets the bias from the SDK, but only when it first makes the context. Setting it here
        // every frame keeps it the temporal curve, as `post::scale::mip_bias` gives for TAA.
        MipBias(crate::post::scale::mip_bias(Antialiasing::Taa, settings.render_scale)),
    ));
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::UpscaleQuality;

    use super::{perf_mode, quality_for_scale};

    #[test]
    fn each_scale_in_the_table_maps_back_to_its_mode() {
        for quality in [
            UpscaleQuality::Native,
            UpscaleQuality::Quality,
            UpscaleQuality::Balanced,
            UpscaleQuality::Performance,
            UpscaleQuality::UltraPerformance,
        ] {
            assert_eq!(quality_for_scale(quality.render_scale()), quality, "{quality:?}");
        }
    }

    #[test]
    fn auto_renders_as_dlss_quality() {
        assert_eq!(quality_for_scale(UpscaleQuality::Auto.render_scale()), UpscaleQuality::Quality);
        assert!(matches!(perf_mode(UpscaleQuality::Auto), bevy_anti_alias::dlss::DlssPerfQualityMode::Quality));
    }
}
