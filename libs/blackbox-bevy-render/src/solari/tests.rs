//! Tests for the Solari path: the startup gate, the caps, the stand-in rules and the mesh preparation (CPU), and
//! one GPU test that starts Solari on a ray-tracing adapter and writes a capture.

use bevy_app::{App, TaskPoolPlugin};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_image::Image;
use bevy_material::OpaqueRendererMethod;
use bevy_mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy_pbr::StandardMaterial;
use bevy_render::renderer::{RenderAdapterInfo, RenderDevice};
use bevy_time::TimePlugin;
use bevy_window::{ExitCondition, WindowPlugin};
use blackbox_gfx::{BlendMode, Denoiser, GraphicsApi, Instance, MeshHandle, RenderBackend, RtSupport, Shading};
use blackbox_gfx_testkit::camera::Camera;
use blackbox_gfx_testkit::mesh::MeshBuilder;
use glam::{Mat4, Vec3};

use super::*;
use crate::apply::instances::{StandInCache, StandIns};
use crate::material::{BlendKind, ShadingKind};
use crate::ops::{BlackboxBridge, Op};
use crate::plugin::{BlackboxBevyRenderPlugin, backend};
use crate::probe::probe_with;

/// The features every Solari path needs.
fn full() -> wgpu::Features {
    SolariPlugins::required_wgpu_features()
}

#[test]
fn the_gate_needs_the_request_an_adapter_and_every_required_feature() {
    assert!(!gate(false, Some(full())), "not requested");
    assert!(!gate(true, None), "no adapter");
    assert!(!gate(true, Some(wgpu::Features::EXPERIMENTAL_RAY_QUERY)), "only the ray query feature");
    assert!(gate(true, Some(full())));
    assert!(gate(true, Some(full() | wgpu::Features::TIMESTAMP_QUERY)), "extra features do not matter");
}

#[test]
fn the_caps_report_ray_tracing_only_when_the_device_has_the_features() {
    assert_eq!(support(wgpu::Features::empty(), false), RtSupport::None);
    assert_eq!(support(wgpu::Features::EXPERIMENTAL_RAY_QUERY, true), RtSupport::None);
    assert_eq!(support(full(), false), RtSupport::Available { denoiser: None });
}

#[test]
fn dlss_ray_reconstruction_is_the_denoiser_only_when_bevy_reports_it() {
    assert_eq!(support(full(), true), RtSupport::Available { denoiser: Some(Denoiser::DlssRr) });
}

#[test]
fn only_opaque_non_sky_draws_become_stand_ins() {
    let cases = [
        (BlendKind::Opaque, ShadingKind::Prelit, true),
        (BlendKind::Opaque, ShadingKind::Lit, true),
        (BlendKind::Opaque, ShadingKind::Glossy, true),
        (BlendKind::AlphaTest, ShadingKind::Prelit, false),
        (BlendKind::AlphaBlend, ShadingKind::Prelit, false),
        (BlendKind::Additive, ShadingKind::Prelit, false),
        (BlendKind::Opaque, ShadingKind::Sky, false),
    ];
    for (blend, shading, want) in cases {
        assert_eq!(eligible(blend, shading), want, "{blend:?} {shading:?}");
    }
}

#[test]
fn a_stand_in_is_one_deferred_material_per_texture_and_carries_the_texture() {
    let mut standard = Assets::<StandardMaterial>::default();
    let mut cache = StandInCache::default();
    let image = Handle::<Image>::default();
    let mut stand_ins = StandIns { standard: &mut standard, cache: &mut cache };

    let first = stand_in(&mut stand_ins, Some(image.clone()), Some(3), BlendKind::Opaque, ShadingKind::Prelit)
        .expect("an opaque draw is a stand-in");
    let again = stand_in(&mut stand_ins, Some(image.clone()), Some(3), BlendKind::Opaque, ShadingKind::Lit)
        .expect("the same texture gives the same stand-in");
    assert_eq!(first, again);

    let other = stand_in(&mut stand_ins, None, None, BlendKind::Opaque, ShadingKind::Prelit);
    assert_ne!(other, Some(first.clone()), "another texture is another material");

    let blended = stand_in(&mut stand_ins, Some(image.clone()), Some(4), BlendKind::AlphaTest, ShadingKind::Prelit);
    assert!(blended.is_none(), "alpha-tested draws keep the Blackbox material");
    let sky = stand_in(&mut stand_ins, Some(image), Some(5), BlendKind::Opaque, ShadingKind::Sky);
    assert!(sky.is_none(), "sky keeps the Blackbox material");

    let material = stand_ins.standard.get(&first).expect("the material is in the store");
    assert_eq!(material.opaque_render_method, OpaqueRendererMethod::Deferred);
    assert!(material.base_color_texture.is_some());
    assert_eq!(stand_ins.cache.0.len(), 2, "only the eligible draws were cached");
}

/// A world mesh the way the crate builds one: u16 indices, no tangents.
fn world_mesh() -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    mesh.insert_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]));
    mesh
}

#[test]
fn a_world_mesh_is_prepared_for_solari() {
    let mut mesh = world_mesh();
    prepare_mesh(&mut mesh);
    assert!(matches!(mesh.indices(), Some(Indices::U32(i)) if i.len() == 6), "u32 indices");
    assert!(mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_some(), "tangents");
    assert!(mesh.enable_raytracing);
    // Preparing again changes nothing.
    prepare_mesh(&mut mesh);
    assert!(matches!(mesh.indices(), Some(Indices::U32(i)) if i.len() == 6));
    let Some(VertexAttributeValues::Float32x4(tangents)) = mesh.attribute(Mesh::ATTRIBUTE_TANGENT) else {
        panic!("tangents are four floats");
    };
    assert_eq!(tangents.len(), 4);
}

#[test]
fn only_mesh_ops_are_prepared() {
    let op = Op::RemoveMesh(MeshHandle::from_raw(1));
    assert!(matches!(prepare_op(op), Op::RemoveMesh(_)));
}

fn write_png(path: &std::path::Path, image: &blackbox_gfx::RgbaImage) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
}

/// Starts Solari on a ray-tracing adapter, draws one lit cuboid off-screen and writes the capture to
/// `BLACKBOX_DUMP_DIR` (or the temp dir). Needs an RTX-class GPU with the Vulkan ray query features. Skips when
/// the adapter is missing or lacks them.
#[test]
#[ignore = "needs a ray-tracing GPU (Vulkan, RTX class)"]
fn solari_starts_and_draws_a_capture_on_a_ray_tracing_gpu() {
    let _gpu = blackbox_gpu_passes::test_support::serial();
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_test_writer()
        .try_init();
    let api = GraphicsApi::Vulkan;
    if let Err(e) = probe_with(api, false) {
        eprintln!("skipping: {e}");
        return;
    }
    if !gate(true, adapter_features(api, false)) {
        eprintln!("skipping: the adapter lacks the Solari features");
        return;
    }
    let size = [640, 360];
    let mut app = App::new();
    app.add_plugins((
        TaskPoolPlugin::default(),
        TimePlugin,
        WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            close_when_requested: false,
            ..WindowPlugin::default()
        },
        BlackboxBevyRenderPlugin { api, ray_tracing: true, ..Default::default() },
    ));
    app.finish();
    app.cleanup();
    assert!(app.world().contains_resource::<StandInCache>(), "install adds the stand-in cache");
    let device = app.world().resource::<RenderDevice>().clone();
    assert!(device.features().contains(full()), "the device has the Solari features");
    let adapter = app.world().resource::<RenderAdapterInfo>().clone();
    let mut backend = backend(app.world().resource::<BlackboxBridge>(), &device, &adapter, size);
    let caps = *backend.capabilities();
    eprintln!("ray tracing caps: {:?}", caps.ray_tracing);
    assert!(caps.ray_tracing.is_available());

    // Lit, not prelit: a prelit draw with no texture does not show in this renderer (checked with the
    // rasterised path, `docs/bevy-backend.md` known gaps). The stand-in ignores the shading anyway.
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Lit).cuboid(Vec3::splat(-1.0), Vec3::splat(1.0), [200, 120, 80, 255]);
    let handle = mesh.finish().upload(&mut backend, "solari_cuboid");
    let instances = [Instance::new(handle, Mat4::IDENTITY)];
    let camera = Camera::new(Vec3::new(4.0, -5.0, 3.0), Vec3::ZERO);
    let frame = camera.frame(size[0] as f32 / size[1] as f32, [0.1, 0.12, 0.2], Some((8.0, 45.0)));

    let id = backend.request_capture(size, &frame, &instances).expect("capture");
    let mut image = None;
    for _ in 0..60 {
        if let Some(result) = backend.poll_capture(id) {
            image = Some(result.expect("capture image"));
            break;
        }
        backend.render(&frame, &instances).expect("render");
        app.update();
    }
    let image = image.expect("the capture finished");
    let dir = std::env::var("BLACKBOX_DUMP_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join("solari_cuboid.png");
    write_png(&path, &image);
    eprintln!("solari capture written to {}", path.display());
    // Open: the picture is only the clear colour on the RTX card (the stand-in draws when forward, so it is
    // spawned; the Solari lighting pass shows nothing). This test checks the start-up and the capture, and
    // prints the colour count so a run can be compared with the fix; it does not assert on the picture yet.
    let distinct: std::collections::HashSet<[u8; 3]> =
        image.rgba.as_chunks::<4>().0.iter().map(|p| [p[0], p[1], p[2]]).collect();
    eprintln!("solari capture: {} distinct colours", distinct.len());
}
