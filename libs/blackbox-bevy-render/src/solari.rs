//! Bevy Solari ray-traced lighting (ADR 0004, PR 12), behind `ray-tracing`.
//!
//! Set at startup (`BlackboxBevyRenderPlugin::ray_tracing`, a restart-required setting) and only when the
//! adapter has the features Solari needs ([`install`]):
//!
//! - the `SolariPlugins` join the app. `DefaultOpaqueRendererMethod` goes back to forward, so the Blackbox
//!   materials keep the forward path; the stand-ins below opt in to deferred on their own;
//! - every camera gets `SolariLighting` and the prepasses it requires ([`sync_cameras`]);
//! - a shadowless sun follows the frame's light direction, because the shadow-only sun in `apply/shadows.rs`
//!   has no radiance for Solari ([`sync_sun`]);
//! - an opaque, non-sky world mesh is drawn as a stand-in: a `RaytracingMesh3d` with a `StandardMaterial` that
//!   carries the base colour texture only ([`stand_in`]). Alpha-tested, blended and additive draws keep the
//!   Blackbox material and are not traced, because Solari 0.20 has no alpha masks (docs/plans/gfx-renderers/
//!   status.md, PR 12);
//! - with the `dlss` build and Bevy's DLSS Ray Reconstruction present, DLSS-RR is the denoiser ([`sync_denoiser`]).
//!   That path is written against Bevy's API and not run: the DLSS SDK is not in the development environment.
//!
//! The stand-ins are a PBR stand-in for a pre-lit world, so the look is different and may disappoint
//! (docs/decisions/0004-swappable-renderers.md, consequences).

use bevy_app::{App, PostUpdate};
use bevy_asset::Handle;
use bevy_camera::primitives::Aabb;
use bevy_camera::visibility::Visibility;
use bevy_camera::{Camera3d, CameraMainTextureUsages, Hdr};
use bevy_core_pipeline::prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Commands, Local, Query, Res};
use bevy_image::Image;
use bevy_light::DirectionalLight;
use bevy_light::light_consts::lux;
use bevy_material::OpaqueRendererMethod;
use bevy_math::Vec3;
use bevy_mesh::{Indices, Mesh, Mesh3d};
use bevy_pbr::{DefaultOpaqueRendererMethod, MeshMaterial3d, StandardMaterial};
use bevy_solari::SolariPlugins;
use bevy_solari::prelude::SolariLighting;
use bevy_solari::scene::RaytracingMesh3d;
use bevy_transform::components::{GlobalTransform, Transform};
use blackbox_gfx::{Denoiser, GraphicsApi, RtSupport};
use wgpu::TextureUsages;

use crate::apply::apply;
use crate::apply::instances::{StandInCache, StandIns};
use crate::material::{BlendKind, ShadingKind};
use crate::ops::{BlackboxBridge, Op};

/// Whether Solari is wanted and the adapter can run it. Pure, so the rule is testable. `adapter` holds the
/// adapter's features, or `None` when there is no adapter.
pub fn gate(requested: bool, adapter: Option<wgpu::Features>) -> bool {
    if !requested {
        return false;
    }
    let Some(features) = adapter else { return false };
    features.contains(SolariPlugins::required_wgpu_features())
}

/// The features of the adapter Bevy would pick for `api`, found before the app exists (Bevy panics when it
/// finds no adapter once the app is being built). Same request as [`crate::probe::probe_with`].
pub fn adapter_features(api: GraphicsApi, force_fallback: bool) -> Option<wgpu::Features> {
    let backends = crate::probe::backends_for(api)?;
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    let request = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: force_fallback,
        compatible_surface: None,
        apply_limit_buckets: false,
    };
    pollster::block_on(instance.request_adapter(&request)).ok().map(|adapter| adapter.features())
}

/// What the device offers for ray tracing, for the caps: available when the device has the Solari features,
/// with DLSS-RR as the denoiser when Bevy's DLSS Ray Reconstruction support is present.
pub fn support(features: wgpu::Features, rr_supported: bool) -> RtSupport {
    if !features.contains(SolariPlugins::required_wgpu_features()) {
        return RtSupport::None;
    }
    RtSupport::Available { denoiser: rr_supported.then_some(Denoiser::DlssRr) }
}

/// Add Solari when [`gate`] allows it and return whether it was added. Called from the plugin's `build` after the
/// render plugin and before the PBR plugin, which reads the result for its prepasses.
pub fn install(app: &mut App, requested: bool, api: GraphicsApi, force_fallback: bool) -> bool {
    let adapter = match requested {
        true => adapter_features(api, force_fallback),
        false => None,
    };
    if !gate(requested, adapter) {
        if requested {
            log::warn!("ray tracing: the adapter lacks the features Solari needs; lighting stays rasterised");
        }
        return false;
    }
    app.add_plugins(SolariPlugins);
    app.insert_resource(DefaultOpaqueRendererMethod::forward());
    app.init_resource::<StandInCache>();
    app.add_systems(PostUpdate, (sync_sun.before(apply), sync_cameras.after(apply)));
    #[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
    app.add_systems(PostUpdate, sync_denoiser.after(apply));
    log::info!("ray tracing: Solari added (the adapter has the ray query and binding array features)");
    true
}

/// Whether a draw can be a stand-in: opaque, and not sky. Alpha-tested, blended and additive draws stay on the
/// Blackbox material (Solari 0.20 has no alpha masks), and sky is drawn unlit and must not block light.
pub fn eligible(blend: BlendKind, shading: ShadingKind) -> bool {
    blend == BlendKind::Opaque && shading != ShadingKind::Sky
}

/// The stand-in material for a draw, or `None` when the draw stays on the Blackbox material. One material per
/// texture number, shared by every draw that uses it; `image` is the texture the number resolves to.
pub fn stand_in(
    stand_ins: &mut StandIns,
    image: Option<Handle<Image>>,
    key: Option<usize>,
    blend: BlendKind,
    shading: ShadingKind,
) -> Option<Handle<StandardMaterial>> {
    if !eligible(blend, shading) {
        return None;
    }
    if let Some(handle) = stand_ins.cache.0.get(&key) {
        return Some(handle.clone());
    }
    let handle = stand_ins.standard.add(StandardMaterial {
        base_color_texture: image,
        opaque_render_method: OpaqueRendererMethod::Deferred,
        ..Default::default()
    });
    stand_ins.cache.0.insert(key, handle.clone());
    Some(handle)
}

/// Spawn one stand-in entity: the rasterised copy that the deferred prepass draws, and the `RaytracingMesh3d`
/// that Solari traces. Both share the mesh. `RaytracingMesh3d` requires `Transform`, so it is set from the
/// global transform; without it Bevy's propagation would reset the position to the identity.
pub fn spawn_stand_in(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    aabb: Aabb,
    transform: GlobalTransform,
) -> Entity {
    let local: Transform = transform.compute_transform();
    commands
        .spawn((
            Mesh3d(mesh.clone()),
            RaytracingMesh3d(mesh),
            MeshMaterial3d(material),
            aabb,
            local,
            transform,
            Visibility::Visible,
        ))
        .id()
}

/// Make a mesh one Solari can trace: `Indices::U32` (Solari's requirement), tangents (its vertex set is
/// POSITION, NORMAL, UV_0 and TANGENT) and the ray-tracing flag. Meshes are prepared before they are added (see
/// [`prepare_op`]), because Bevy drops a `RENDER_WORLD` mesh from the main world once it is extracted.
pub fn prepare_mesh(mesh: &mut Mesh) {
    mesh.enable_raytracing = true;
    let wide = match mesh.indices() {
        Some(Indices::U16(indices)) => Some(indices.iter().map(|&i| u32::from(i)).collect::<Vec<u32>>()),
        _ => None,
    };
    if let Some(wide) = wide {
        mesh.insert_indices(Indices::U32(wide));
    }
    if mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_some() {
        return;
    }
    if let Err(e) = mesh.generate_tangents() {
        log::warn!("ray tracing: tangents for a world mesh failed ({e}); Solari may not trace it");
    }
}

/// Prepare the meshes of an `AddMesh` op for Solari. Every other op passes through unchanged.
pub fn prepare_op(op: Op) -> Op {
    match op {
        Op::AddMesh { handle, mut ranges } => {
            for range in &mut ranges {
                prepare_mesh(&mut range.mesh);
            }
            Op::AddMesh { handle, ranges }
        }
        other => other,
    }
}

/// The frame's light direction, from the frame that is about to be applied or from the first capture waiting.
fn light_dir(bridge: &BlackboxBridge) -> Option<glam::Vec3> {
    let shared = bridge.lock();
    shared.frame.as_ref().map(|f| f.frame.light_dir).or_else(|| shared.captures.front().map(|c| c.data.frame.light_dir))
}

/// The Solari sun: one shadowless directional light along the frame's light direction. It is spawned once and
/// turned when the direction changes. Runs before `apply`, which consumes the frame.
fn sync_sun(mut commands: Commands, bridge: Res<BlackboxBridge>, mut sun: Local<Option<(Entity, Vec3)>>) {
    let Some(light_dir) = light_dir(&bridge) else { return };
    let direction = crate::axes::point(light_dir);
    let direction = Vec3::from_array(direction.to_array()).try_normalize().unwrap_or(Vec3::NEG_Y);
    let transform = Transform::default().looking_to(direction, Vec3::Z);
    if let Some((entity, last)) = *sun {
        if last == direction {
            return;
        }
        commands.entity(entity).insert((transform, GlobalTransform::from(transform)));
        *sun = Some((entity, direction));
        return;
    }
    let light =
        DirectionalLight { illuminance: lux::AMBIENT_DAYLIGHT, shadow_maps_enabled: false, ..Default::default() };
    let entity = commands.spawn((light, transform, GlobalTransform::from(transform))).id();
    *sun = Some((entity, direction));
}

/// Every camera draws with Solari. Components are added only when missing. `apply` may remove the prepasses or
/// HDR when the post settings change, and this puts them back after it. Solari writes the main texture from a
/// compute shader, so that texture needs `STORAGE_BINDING` (Solari's own requirement). The temporal history is
/// not reset on a camera cut yet (docs/bevy-backend.md, known gaps).
fn sync_cameras(mut commands: Commands, cameras: Query<Entity, With<Camera3d>>) {
    let main_texture = CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING);
    for camera in &cameras {
        commands.entity(camera).insert_if_new((
            SolariLighting::default(),
            DeferredPrepass,
            DepthPrepass,
            MotionVectorPrepass,
            Hdr,
            main_texture,
        ));
    }
}

/// DLSS Ray Reconstruction on every camera, once Bevy reports it. Not run in the development environment (no
/// DLSS SDK).
#[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
fn sync_denoiser(
    mut commands: Commands,
    cameras: Query<Entity, With<Camera3d>>,
    rr: Option<Res<bevy_anti_alias::dlss::DlssRayReconstructionSupported>>,
) {
    if rr.is_none() {
        return;
    }
    for camera in &cameras {
        let denoiser = bevy_anti_alias::dlss::Dlss::<bevy_anti_alias::dlss::DlssRayReconstructionFeature>::default();
        commands.entity(camera).insert_if_new(denoiser);
    }
}

#[cfg(test)]
mod tests;
