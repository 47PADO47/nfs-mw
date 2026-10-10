//! Temporal anti-aliasing: inserting and removing `TemporalAntiAliasing`, and resetting its history on a
//! camera cut.
//!
//! `TemporalAntiAliasing` requires `TemporalJitter`, `MipBias`, `DepthPrepass` and `MotionVectorPrepass`
//! (its `#[require(...)]`), so inserting it is enough to turn jittering and both prepasses on. Bevy does
//! not remove required components when the requiring one is removed, so turning TAA off removes them
//! explicitly too — otherwise a camera that once had TAA on would keep paying for the prepasses forever,
//! and the "no prepass components when TAA is off" test would only pass for a camera that never had it.
//!
//! Motion vectors need no wiring here: `bevy_pbr`'s `update_mesh_previous_global_transforms` (in
//! `PreUpdate`, every frame) keeps every `Mesh3d` entity's `PreviousGlobalTransform` one frame behind its
//! `GlobalTransform`, driven by Bevy's own change detection. Since `apply/instances.rs` only writes
//! `GlobalTransform` on an instance that actually moved, and reuses the same `Entity` across frames for a
//! given `InstanceKey` (no respawn), a still object's `PreviousGlobalTransform` stays equal to its current
//! transform (no spurious motion) and a moving one gets a correct one-frame-old value: free, correct
//! motion vectors from the instance pool's existing stability guarantee.

use bevy_anti_alias::taa::TemporalAntiAliasing;
use bevy_core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass};
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Commands;
use bevy_render::camera::TemporalJitter;

/// Turn TAA on (if it was off) or off (removing every component it requires) on `camera`.
///
/// Turning it on when it is already on is a no-op (via `entry().or_insert_with`, not a plain `insert`):
/// a fresh `TemporalAntiAliasing::default()` has `reset: true`, so overwriting it every frame while TAA
/// stays on would wipe the history every frame and TAA would never converge.
pub fn set(commands: &mut Commands, camera: Entity, enabled: bool) {
    let mut entity = commands.entity(camera);
    match enabled {
        true => {
            entity.entry::<TemporalAntiAliasing>().or_default();
        }
        false => {
            entity.remove::<(TemporalAntiAliasing, DepthPrepass, MotionVectorPrepass, TemporalJitter)>();
        }
    }
}

/// Reset the history on a camera cut (a teleport, a freecam toggle, a scene switch), so the next frame
/// does not blend against a picture of somewhere else. A no-op when TAA is off or this frame is not a cut;
/// Bevy clears `reset` back to `false` itself once it has used it (`extract_taa_settings`), so this only
/// ever needs to set it, never clear it.
pub fn reset_on_cut(commands: &mut Commands, camera: Entity, enabled: bool, camera_cut: bool) {
    if !enabled || !camera_cut {
        return;
    }
    commands.entity(camera).entry::<TemporalAntiAliasing>().and_modify(|mut taa| taa.reset = true);
}
