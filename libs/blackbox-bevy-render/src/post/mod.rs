//! Post-process components by the effective [`PostSettings`](blackbox_gfx::PostSettings) and
//! [`Upscaler`](blackbox_gfx::Upscaler): bloom, tone mapping, FXAA, SMAA, TAA (`taa`), the render-scale
//! texture LOD bias (`scale`) and the FSR 1 pass (`fsr1`).
//!
//! `apply/camera.rs` calls [`set`] whenever the effective [`CameraSettings`] change, and on every newly
//! spawned camera (the screen camera and each capture's), so a camera starts in the settings it was asked
//! for instead of catching up a frame later. Every component here is inserted only when its effect is
//! enabled and removed otherwise (`docs/plans/gfx-renderers/upscalers-settings.md`, §5: "nothing runs or
//! is allocated when off"): no `Hdr`, `Bloom`, `TemporalAntiAliasing` or its prepasses when nothing needs
//! them.
//!
//! SMAA is also a plain drop-in `Smaa` component in Bevy 0.20 (`bevy_anti_alias::smaa`), already built by
//! the `AntiAliasPlugin` added in `plugin.rs`; it needed no spike or fallback, unlike the plan's "check
//! its actual source first" caveat for it suggested might be necessary.

pub mod fsr1;
pub mod scale;
pub mod taa;

use bevy_anti_alias::fxaa::Fxaa;
use bevy_anti_alias::smaa::Smaa;
use bevy_camera::Hdr;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Commands;
use bevy_post_process::bloom::{Bloom, BloomPrefilter};
use bevy_render::view::Tonemapping;
use blackbox_gfx::{Antialiasing, PostSettings, Tonemap};

use crate::ops::CameraSettings;

/// How much the thresholded and non-thresholded colours blend into each other, like native's bloom (see
/// `PostSettings::bloom_threshold`'s doc comment: "a soft knee starts a little below").
const BLOOM_THRESHOLD_SOFTNESS: f32 = 0.2;

fn tonemapping(tonemap: Tonemap) -> Tonemapping {
    match tonemap {
        Tonemap::Off => Tonemapping::None,
        Tonemap::Aces => Tonemapping::AcesFitted,
    }
}

/// `Bloom` when the intensity is above zero, built from [`PostSettings::bloom_intensity`] and
/// [`PostSettings::bloom_threshold`] on top of Bevy's natural preset (energy-conserving compositing).
fn bloom(post: &PostSettings) -> Option<Bloom> {
    if post.bloom_intensity <= 0.0 {
        return None;
    }
    Some(Bloom {
        intensity: post.bloom_intensity,
        prefilter: BloomPrefilter { threshold: post.bloom_threshold, threshold_softness: BLOOM_THRESHOLD_SOFTNESS },
        ..Bloom::NATURAL
    })
}

/// Insert or remove every post-process component on `camera` by `settings`. Idempotent: calling it again
/// with the same settings re-applies the same values (cheap, and needed so a freshly spawned camera can
/// be brought to the current settings at once without a separate "first time" path), except TAA, whose
/// own `set` only touches the component when turning it on or off (see `taa::set`).
pub fn set(commands: &mut Commands, camera: Entity, settings: &CameraSettings) {
    let mut entity = commands.entity(camera);
    entity.insert(tonemapping(settings.post.tonemap));
    match settings.post.needs_hdr() {
        true => entity.insert(Hdr),
        false => entity.remove::<Hdr>(),
    };
    match bloom(&settings.post) {
        Some(bloom) => entity.insert(bloom),
        None => entity.remove::<Bloom>(),
    };
    match settings.post.antialiasing {
        Antialiasing::Fxaa => entity.insert(Fxaa::default()),
        _ => entity.remove::<Fxaa>(),
    };
    match settings.post.antialiasing {
        Antialiasing::Smaa => entity.insert(Smaa::default()),
        _ => entity.remove::<Smaa>(),
    };
    taa::set(commands, camera, settings.post.antialiasing == Antialiasing::Taa);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_aces_tone_maps_and_only_positive_intensity_blooms() {
        assert_eq!(tonemapping(Tonemap::Off), Tonemapping::None);
        assert_eq!(tonemapping(Tonemap::Aces), Tonemapping::AcesFitted);
        assert!(bloom(&PostSettings::default()).is_none());
        let bloom = bloom(&PostSettings { bloom_intensity: 0.5, bloom_threshold: 1.2, ..PostSettings::default() })
            .expect("positive intensity blooms");
        assert_eq!(bloom.intensity, 0.5);
        assert_eq!(bloom.prefilter.threshold, 1.2);
    }
}
