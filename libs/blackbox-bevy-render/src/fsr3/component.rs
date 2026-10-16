//! The main-world side of FSR 3: the components a camera needs while FSR 3 is its upscaler, put on or taken off
//! every frame (`apply/mod.rs`'s `drive_screen`, after `post::set`, the same way `dlss::sdk::set` works).

use bevy_camera::{Camera, Hdr};
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::{Commands, ResMut};
use bevy_render::MainWorld;
use bevy_render::camera::{MipBias, TemporalJitter};
use bevy_render::sync_world::RenderEntity;
use blackbox_gfx::{Upscaler, suggested_temporal_texture_lod_bias};

use crate::ops::CameraSettings;

/// Marks a camera that FSR 3 upscales, and carries the per-frame state the render world needs from the main world.
///
/// It is copied to the render world every frame by [`extract`], so the upscale node reads `reset` from it.
/// Its presence is also how the render-scale pass tells the screen camera from a capture camera: only this one is
/// drawn at the render size and upscaled (`post::scale::apply_resolution_override`).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fsr3Upscale {
    /// Drop the upscaler's history this frame: the camera was moved discontinuously (a cut, a teleport, a scene switch).
    pub reset: bool,
}

/// Copy the `Fsr3Upscale` of every active camera to its render-world twin, every frame, and remove it from a camera
/// that no longer has one. Done by hand, like TAA's `extract_taa_settings`: the generic sync path
/// (`SyncComponentPlugin`) did not put the marker on the render view, so the upscale node never ran.
pub fn extract(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras = main_world.query::<(RenderEntity, &Camera, Option<&Fsr3Upscale>)>();
    for (entity, camera, upscale) in cameras.iter_mut(&mut main_world) {
        let Ok(mut render) = commands.get_entity(entity) else { continue };
        match upscale {
            Some(upscale) if camera.is_active => render.insert(*upscale),
            _ => render.remove::<Fsr3Upscale>(),
        };
    }
}

/// Whether FSR 3 is the upscaler the camera runs with.
pub fn wanted(settings: &CameraSettings) -> bool {
    settings.upscaler == Upscaler::Fsr3
}

/// Put the FSR 3 state on `camera` while FSR 3 is the upscaler, and take it off otherwise.
///
/// While FSR 3 is on, the camera gets everything the upscaler reads: the depth and motion-vector prepasses (Bevy
/// writes the motion vectors only when `MotionVectorPrepass` is there), the jitter, `Hdr` (the colour is linear HDR,
/// before tone mapping), and the temporal texture bias (one mip sharper than the spatial one, from
/// `suggested_temporal_texture_lod_bias`). Anti-aliasing is off on this path (`caps::resolve` turns TAA off for any
/// temporal upscaler), so no TAA component is here to conflict with it.
///
/// When FSR 3 is off, only the `Fsr3Upscale` marker goes. The prepasses and the jitter are not removed here: they are
/// `post::taa::set`'s to manage (it removes them whenever TAA is off), and DLSS and Solari need some of them too.
pub fn set(commands: &mut Commands, camera: Entity, settings: &CameraSettings, camera_cut: bool) {
    let mut entity = commands.entity(camera);
    if !wanted(settings) {
        entity.remove::<Fsr3Upscale>();
        return;
    }
    entity.insert((
        Fsr3Upscale { reset: camera_cut },
        DepthPrepass,
        MotionVectorPrepass,
        TemporalJitter::default(),
        Hdr,
        MipBias(suggested_temporal_texture_lod_bias(settings.render_scale)),
    ));
}
