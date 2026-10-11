//! The render scale: the texture LOD bias it suggests, and `MainPassResolutionOverride`, the component
//! that actually makes the main pass draw smaller than the surface.

use bevy_camera::MainPassResolutionOverride;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Commands, Res};
use bevy_render::camera::ExtractedCamera;
use bevy_render::renderer::ViewQuery;
use blackbox_gfx::{Antialiasing, scaled_size, suggested_temporal_texture_lod_bias, suggested_texture_lod_bias};

use crate::ops::BlackboxBridge;

/// The texture LOD bias that keeps mip selection matched to the render scale: one mip sharper while a
/// temporal method (today, only [`Antialiasing::Taa`]) accumulates samples over time, the plain spatial
/// bias otherwise. `libs/blackbox-gfx` computes both curves; this just picks between them.
///
/// A temporal *upscaler* (FSR 3/4, DLSS) would use the same temporal curve, but none of them are offered
/// by this renderer yet (`caps.rs`), so [`Upscaler::is_temporal`](blackbox_gfx::Upscaler::is_temporal)
/// never matters here today. The day one lands, pass its `effective.upscaler` in here too.
pub fn mip_bias(antialiasing: Antialiasing, render_scale: f32) -> f32 {
    match antialiasing {
        Antialiasing::Taa => suggested_temporal_texture_lod_bias(render_scale),
        _ => suggested_texture_lod_bias(render_scale),
    }
}

/// The pixel size the main pass draws at when it is not `target`'s own (`None` at a render scale of 1 or
/// above: `MainPassResolutionOverride` only shrinks the main pass, there is nothing to ask for above
/// native, and a render scale above 1 is supersampling, handled by the upscaler's own resolve instead).
pub fn render_size(target: [u32; 2], render_scale: f32) -> Option<(u32, u32)> {
    if render_scale >= 1.0 {
        return None;
    }
    Some(scaled_size((target[0], target[1]), render_scale))
}

/// Keep every render-world camera's `MainPassResolutionOverride` in step with the render scale.
///
/// This is the one post component that cannot be set from the main world at all: its own doc comment
/// says to insert it "on a 3d camera entity in the render world", and nothing syncs it there
/// automatically (no `ExtractComponentPlugin` registers it, unlike `Bloom`, `Fxaa`, `Smaa` and
/// `TemporalAntiAliasing`, which all do). An earlier version of this renderer inserted it from
/// `apply/camera.rs` in the main world, which compiled and looked right but was silently a no-op: Bevy's
/// own main pass never saw it, so the scene always drew at the full surface size regardless of the
/// render scale — caught by `post_tests::fsr1_matches_native_within_tolerance_at_67_percent`, which
/// (before this fix) measured a softer-looking but **wrongly framed** image, not just a blurrier one:
/// EASU was upscaling a render that was never actually downscaled, from the wrong assumed input size.
/// Scheduled in `Core3dSystems::Prepass`, before the main pass whose size it controls.
pub fn apply_resolution_override(
    bridge: Res<BlackboxBridge>,
    view: ViewQuery<(Entity, &ExtractedCamera)>,
    mut commands: Commands,
) {
    let (entity, camera) = view.into_inner();
    let Some(target) = camera.physical_target_size else { return };
    let render_scale = bridge.lock().settings.render_scale;
    let mut entity = commands.entity(entity);
    match render_size([target.x, target.y], render_scale) {
        Some((w, h)) => entity.insert(MainPassResolutionOverride(bevy_math::UVec2::new(w, h))),
        None => entity.remove::<MainPassResolutionOverride>(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taa_uses_the_temporal_curve_everything_else_the_spatial_one() {
        for scale in [1.0, 0.67, 0.5, 0.33] {
            assert_eq!(mip_bias(Antialiasing::Taa, scale), suggested_temporal_texture_lod_bias(scale));
            for aa in [Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa] {
                assert_eq!(mip_bias(aa, scale), suggested_texture_lod_bias(scale), "{aa:?}");
            }
        }
    }
}
