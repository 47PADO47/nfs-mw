//! Per-frame CPU and GPU cost of the native renderer in the real world scene, with and without the rear-view
//! mirror. Each frame runs in the app's order: scene update, then the mirror (when on), then scene frame and render.
use super::*;
use crate::app::mirror::MirrorOverlay;
use blackbox_render::{Renderer, RendererOptions};
use std::time::{Duration, Instant};

/// Full HD, the size the frame is drawn at.
const SIZE: (u32, u32) = (1920, 1080);
/// Frames drawn before timing starts, so streaming, caches and the mirror's first capture settle.
const WARM_UP_FRAMES: usize = 30;
/// Timed frames per run.
const MEASURED_FRAMES: usize = 200;
/// Simulation ticks after streaming, for placement and chase-camera convergence.
const SETTLE_TICKS: usize = 120;

#[test]
#[ignore = "needs the game and a GPU; diagnostic benchmark"]
fn world_frame_costs() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let api = match std::env::var("BLACKBOX_TEST_API").as_deref() {
        Ok("dx12") => blackbox_gfx::GraphicsApi::Dx12,
        _ => blackbox_gfx::GraphicsApi::Vulkan,
    };
    for (name, at) in [("hill", [556.0, 1802.0]), ("city", [2150.0, 1399.0])] {
        for mirror_on in [false, true] {
            let (mut cpu, mut gpu) = measure(&dir, api, at, mirror_on);
            let p50 = percentile(&mut cpu, 50);
            let p95 = percentile(&mut cpu, 95);
            eprintln!(
                "frame-cost {api:?} {name} mirror={mirror_on}: CPU p50 {p50:.3} ms p95 {p95:.3} ms, GPU p50 {}",
                gpu_p50_text(&mut gpu)
            );
        }
    }
}

/// Sets up one run and times its measured frames. Returns the CPU ms of each timed frame, and the GPU ms of each
/// frame the renderer reported (none when timestamps are unsupported).
fn measure(dir: &GameDir, api: blackbox_gfx::GraphicsApi, at: [f32; 2], mirror_on: bool) -> (Vec<f32>, Vec<f32>) {
    let options = RendererOptions { backend: api, vsync: false, force_fallback_adapter: false };
    let mut renderer = Renderer::headless(SIZE, options).unwrap();
    let mut world = WorldScene::open(
        dir,
        Options {
            at: Some(at),
            height: 40.0,
            heading: 45.0,
            pitch: -20.0,
            fog_distance: 3000.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: DEFAULT_CAR.into(), script: None }),
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    wait_ready(&mut world, &mut renderer);
    let input = ActionState::default();
    for _ in 0..SETTLE_TICKS {
        world.update(&mut renderer, &input, 1.0 / 60.0);
    }
    wait_ready(&mut world, &mut renderer);
    world.update(&mut renderer, &input, 0.0);

    let mut mirror = MirrorOverlay::default();
    let aspect = SIZE.0 as f32 / SIZE.1 as f32;
    let mut cpu = Vec::with_capacity(MEASURED_FRAMES);
    let mut gpu = Vec::with_capacity(MEASURED_FRAMES);
    for index in 0..WARM_UP_FRAMES + MEASURED_FRAMES {
        let t = Instant::now();
        world.update(&mut renderer, &input, 1.0 / 60.0);
        if mirror_on {
            mirror.update(&mut renderer, &mut world);
        }
        let (frame, instances) = world.frame(aspect);
        renderer.render(&frame, instances).unwrap();
        let elapsed = t.elapsed();
        if index < WARM_UP_FRAMES {
            continue;
        }
        cpu.push(elapsed.as_secs_f32() * 1000.0);
        // The GPU sample is the latest completed timestamp readback, so it can trail the frame by a few frames.
        if let Some(ms) = renderer.experiment_stats().gpu_frame_ms {
            gpu.push(ms);
        }
    }
    (cpu, gpu)
}

/// Steps the world without advancing time until it has streamed what it needs.
fn wait_ready(world: &mut WorldScene, renderer: &mut Renderer) {
    let start = Instant::now();
    while !world.ready() {
        world.update(renderer, &ActionState::default(), 0.0);
        assert!(start.elapsed() < Duration::from_secs(60), "streaming timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The GPU p50 as text, or "n/a" when the renderer reported no timestamps.
fn gpu_p50_text(gpu: &mut [f32]) -> String {
    if gpu.is_empty() {
        return "n/a".to_string();
    }
    format!("{:.3} ms", percentile(gpu, 50))
}

/// Nearest-rank percentile `p` (0 to 100) of `values`, which it sorts. `values` must not be empty.
fn percentile(values: &mut [f32], p: usize) -> f32 {
    values.sort_unstable_by(f32::total_cmp);
    let rank = (p * values.len()).div_ceil(100).clamp(1, values.len());
    values[rank - 1]
}
