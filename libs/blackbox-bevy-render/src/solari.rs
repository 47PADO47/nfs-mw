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
use bevy_color::Color;
use bevy_core_pipeline::prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Commands, Local, Query, Res, ResMut};
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
use crate::mesh::ATTRIBUTE_COLOR_BGRA;
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
    app.init_resource::<CameraCut>();
    app.add_systems(PostUpdate, (sync_sun.before(apply), note_camera_cut.before(apply), sync_cameras.after(apply)));
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

/// The colour a stand-in is keyed and drawn with: each gamma byte moves to the centre of its 16-step bucket, so
/// ranges with nearly the same colour share one material.
pub fn quantise(colour: [u8; 3]) -> [u8; 3] {
    colour.map(|c| (c & 0xF0) | 0x08)
}

/// The stand-in material for a draw, or `None` when the draw stays on the Blackbox material. One material per
/// texture number and quantised colour, shared by every draw that uses them; `image` is the texture the number
/// resolves to, and `colour` the range's average vertex colour.
pub fn stand_in(
    stand_ins: &mut StandIns,
    image: Option<Handle<Image>>,
    key: Option<usize>,
    colour: [u8; 3],
    blend: BlendKind,
    shading: ShadingKind,
) -> Option<Handle<StandardMaterial>> {
    if !eligible(blend, shading) {
        return None;
    }
    let colour = quantise(colour);
    if let Some(handle) = stand_ins.cache.0.get(&(key, colour)) {
        return Some(handle.clone());
    }
    let handle = stand_ins.standard.add(stand_in_material(image, colour, shading));
    stand_ins.cache.0.insert((key, colour), handle.clone());
    Some(handle)
}

/// The PBR material for a stand-in. The Blackbox shader multiplies the texture by the vertex colour (in gamma
/// space), so a lit stand-in takes the colour as its base colour and keeps the texture. Solari has no vertex colour,
/// so the average stands in for it. A pre-lit draw has no direct light of its own: it is emitted instead, with the
/// shader's pre-lit gain of 2 applied and clamped, and its base is black so the sun adds nothing on top. The
/// emission is not calibrated against the sun (an approximation, `docs/plans/gfx-renderers/status.md` 3d).
fn stand_in_material(image: Option<Handle<Image>>, colour: [u8; 3], shading: ShadingKind) -> StandardMaterial {
    let [r, g, b] = colour.map(|c| f32::from(c) / 255.0);
    let opaque_render_method = OpaqueRendererMethod::Deferred;
    match shading {
        ShadingKind::Prelit => {
            let emitted = Color::srgb((r * 2.0).min(1.0), (g * 2.0).min(1.0), (b * 2.0).min(1.0)).to_linear();
            StandardMaterial {
                base_color: Color::BLACK,
                emissive: emitted,
                emissive_texture: image,
                opaque_render_method,
                ..Default::default()
            }
        }
        _ => StandardMaterial {
            base_color: Color::srgb(r, g, b),
            base_color_texture: image,
            opaque_render_method,
            ..Default::default()
        },
    }
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

/// Make a mesh one Solari can trace: `Indices::U32` (Solari's requirement), tangents, and the ray-tracing flag.
/// Solari accepts a mesh only when its attributes are exactly POSITION, NORMAL, UV_0 and TANGENT (Bevy's
/// `is_mesh_raytracing_compatible`), so the Blackbox vertex colour goes: a stand-in never draws the Blackbox
/// material, which is the only thing that reads it. Meshes are prepared before they are added (see
/// [`prepare_op`]), because Bevy drops a `RENDER_WORLD` mesh from the main world once it is extracted.
pub fn prepare_mesh(mesh: &mut Mesh) {
    mesh.enable_raytracing = true;
    mesh.remove_attribute(ATTRIBUTE_COLOR_BGRA.id);
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

/// Prepare the ranges of an `AddMesh` op that a stand-in draws (see [`eligible`]). A range that stays on the
/// Blackbox material keeps its vertex colour, because that material needs it. Every other op passes through.
pub fn prepare_op(op: Op) -> Op {
    match op {
        Op::AddMesh { handle, mut ranges } => {
            for range in &mut ranges {
                let blend = BlendKind::from(range.blend);
                let shading = ShadingKind::of(range.shading);
                if !eligible(blend, shading) {
                    continue;
                }
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

/// Whether this tick's frame is a camera cut. `apply` takes the frame, so this is read before it ([`note_camera_cut`]).
#[derive(Resource, Default)]
struct CameraCut(bool);

/// Read the frame's `camera_cut` flag while the frame is still in the bridge, before `apply` takes it.
fn note_camera_cut(bridge: Res<BlackboxBridge>, mut cut: ResMut<CameraCut>) {
    cut.0 = bridge.lock().frame.as_ref().is_some_and(|f| f.frame.camera_cut);
}

/// Every camera draws with Solari. Components are added only when missing. `apply` may remove the prepasses or
/// HDR when the post settings change, and this puts them back after it. Solari writes the main texture from a
/// compute shader, so that texture needs `STORAGE_BINDING` (Solari's own requirement). `Camera` already requires a
/// default `CameraMainTextureUsages` without it, so that one is set when the flag is missing rather than inserted
/// only when absent. On a camera cut the temporal history is reset, so the next frame does not blend with the old view.
fn sync_cameras(
    mut commands: Commands,
    cameras: Query<(Entity, Option<&CameraMainTextureUsages>), With<Camera3d>>,
    cut: Res<CameraCut>,
) {
    for (camera, usages) in &cameras {
        commands.entity(camera).insert_if_new((
            SolariLighting::default(),
            DeferredPrepass,
            DepthPrepass,
            MotionVectorPrepass,
            Hdr,
        ));
        if !has_storage(usages) {
            let main_texture = CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING);
            commands.entity(camera).insert(main_texture);
        }
        reset_on_cut(&mut commands, camera, cut.0);
    }
}

/// Whether a camera's main texture already has the storage usage Solari writes through.
fn has_storage(usages: Option<&CameraMainTextureUsages>) -> bool {
    usages.is_some_and(|usages| usages.0.contains(TextureUsages::STORAGE_BINDING))
}

/// Reset the Solari history on a camera cut, the way `post::taa::reset_on_cut` does for TAA. A no-op when this frame
/// is not a cut. Solari clears `reset` itself at extraction (`extract_solari_lighting`), so this only sets it.
pub fn reset_on_cut(commands: &mut Commands, camera: Entity, camera_cut: bool) {
    if !camera_cut {
        return;
    }
    commands.entity(camera).entry::<SolariLighting>().and_modify(|mut solari| solari.reset = true);
}

/// DLSS Ray Reconstruction on every window camera, once Bevy reports it. Capture cameras (render target = image)
/// never run DLSS (`apply/capture.rs`). Not run in the development environment (no DLSS SDK).
#[cfg(all(feature = "dlss", not(feature = "dlss-mock")))]
fn sync_denoiser(
    mut commands: Commands,
    cameras: Query<(Entity, &bevy_camera::RenderTarget), With<Camera3d>>,
    rr: Option<Res<bevy_anti_alias::dlss::DlssRayReconstructionSupported>>,
) {
    if rr.is_none() {
        return;
    }
    for (camera, target) in &cameras {
        if !matches!(target, bevy_camera::RenderTarget::Window(_)) {
            continue;
        }
        let denoiser = bevy_anti_alias::dlss::Dlss::<bevy_anti_alias::dlss::DlssRayReconstructionFeature>::default();
        commands.entity(camera).insert_if_new(denoiser);
    }
}

#[cfg(test)]
mod tests;
