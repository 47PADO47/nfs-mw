//! How the cost of a frame grows with the number of placed objects. A GPU test that prints, not asserts: it is
//! the evidence for the instancing section of `docs/bevy-backend.md`.

use blackbox_gfx::{
    BlendMode, DrawRange, GraphicsApi, Instance, InstanceKey, MeshDesc, PixelFormat, RenderBackend, Shading,
    TextureDesc, Vertex,
};
use blackbox_gfx_testkit::camera::Camera;
use blackbox_gpu_passes::test_support::serial;
use glam::{Mat4, Vec3};

use crate::HeadlessBevy;

/// Milliseconds per frame (Bevy's update included, CPU and GPU together) for `counts` keyed, static instances of
/// one single-range mesh, or `None` without an adapter.
fn frame_cost(counts: &[u32]) -> Option<Vec<(u32, f64)>> {
    let fallback = std::env::var("BLACKBOX_GPU_FALLBACK").is_ok_and(|v| !v.is_empty() && v != "0");
    let mut bevy = HeadlessBevy::new([640, 360], GraphicsApi::Vulkan, fallback).ok()?;
    let pixels = [200u8; 16];
    let texture = bevy.create_texture(&TextureDesc {
        label: "stress",
        width: 2,
        height: 2,
        format: PixelFormat::Rgba8,
        mips: vec![&pixels],
    });
    let v =
        |x: f32, y: f32| Vertex { position: [x, y, 0.0], normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv: [x, y] };
    let vertices = [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0)];
    let draw = DrawRange {
        first_index: 0,
        index_count: 3,
        base_vertex: 0,
        texture: Some(texture),
        blend: BlendMode::Opaque,
        shading: Shading::Prelit,
    };
    let mesh =
        bevy.create_mesh(&MeshDesc { label: "tri", vertices: &vertices, indices: &[0, 1, 2], draws: vec![draw] });
    let frame = Camera::new(Vec3::new(0.0, -30.0, 20.0), Vec3::new(0.0, 100.0, 0.0)).frame(16.0 / 9.0, [0.5; 3], None);
    let mut results = Vec::new();
    for &count in counts {
        let instances: Vec<Instance> = (0..count)
            .map(|i| {
                let at = Vec3::new((i % 200) as f32 * 1.5 - 150.0, (i / 200) as f32 * 1.5, 0.0);
                Instance::keyed(mesh, Mat4::from_translation(at), InstanceKey::new(1, i))
            })
            .collect();
        // A camera exists while a capture runs, so every pump of the app draws the scene.
        let mut pumps = 0u32;
        let mut elapsed = std::time::Duration::ZERO;
        for round in 0..3 {
            let id = bevy.request_capture([640, 360], &frame, &instances).ok()?;
            while bevy.poll_capture(id).is_none() {
                let start = std::time::Instant::now();
                bevy.render(&frame, &instances).ok()?;
                // The first capture warms the pipelines up.
                if round > 0 {
                    elapsed += start.elapsed();
                    pumps += 1;
                }
            }
        }
        results.push((count, elapsed.as_secs_f64() * 1000.0 / f64::from(pumps)));
    }
    Some(results)
}

#[test]
#[ignore = "needs a GPU"]
fn the_frame_cost_grows_with_the_instance_count() {
    let _gpu = serial();
    let Some(results) = frame_cost(&[1_000, 10_000, 40_000]) else { return };
    for (count, ms) in results {
        eprintln!("{count} instances: {ms:.2} ms per frame");
    }
}
