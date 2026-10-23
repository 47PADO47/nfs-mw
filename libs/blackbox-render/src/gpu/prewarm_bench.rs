//! How long a prewarm takes with an empty pipeline cache, for the numbers in the performance notes.
//!
//! `cargo test -p blackbox-render --lib prewarm_bench -- --ignored --nocapture`. The Vulkan pipeline cache starts
//! empty (a fresh directory per run), but the driver keeps its own shader cache: on NVIDIA set
//! `__GL_SHADER_DISK_CACHE=0` (or point `__GL_SHADER_DISK_CACHE_PATH` at an empty directory) for a really cold run.

use std::time::Instant;

use super::{Renderer, test_support};
use crate::RendererOptions;
use blackbox_gfx::*;

/// Everything on that the headless renderer can run: every family, the post chain, the reflection cube, shadows
/// and AO, so the prewarm builds as much as a high setting in the game does.
fn settings(caps: &Capabilities) -> GraphicsSettings {
    let upscaler = match caps.upscalers.contains(Upscaler::Fsr3) {
        true => Upscaler::Fsr3,
        false => Upscaler::Fsr1,
    };
    GraphicsSettings {
        post: PostSettings {
            colour_treatment: 1.0,
            tonemap: Tonemap::Aces,
            bloom_intensity: 0.5,
            antialiasing: Antialiasing::Smaa,
            ..PostSettings::default()
        },
        reflections: ReflectionQuality::High,
        ao: AoSettings { quality: AoQuality::High, ..AoSettings::default() },
        shadows: ShadowQuality::High,
        upscaler,
        upscale_quality: UpscaleQuality::Quality,
        ..GraphicsSettings::default()
    }
}

fn texel(r: &mut Renderer, label: &str, rgba: &[u8; 4]) -> TextureHandle {
    r.create_texture(&TextureDesc { label, width: 1, height: 1, format: PixelFormat::Rgba8, mips: vec![rgba] })
}

/// One material of every family, so the sky, normal, window and glossy pipelines are part of the prewarm.
fn add_families(r: &mut Renderer) {
    let diffuse = texel(r, "diffuse", &[128, 128, 128, 255]);
    let flat = texel(r, "flat", &[128, 128, 255, 255]);
    let black = texel(r, "black", &[0, 0, 0, 255]);
    for model in [NormalModel::Bump, NormalModel::Reflective] {
        let desc =
            NormalMaterialDesc { model, diffuse, normal: flat, specular: black, reflection: None, uv_offset: [0.0; 2] };
        r.create_normal_material(&desc).expect("normal material");
    }
    r.create_window_material(&WindowMaterialDesc { diffuse, reflection: black }).expect("window material");
    let sky = SkyMaterialDesc { diffuse, miscellaneous: diffuse, kind: SkyLayerKind::Cloud2 };
    r.create_sky_material(&sky).expect("sky material");
    r.create_glossy_material(&GlossyMaterial::default());
}

/// A renderer with an empty pipeline cache in `dir`, every family and the high settings, ready to prewarm.
fn cold_renderer(dir: &std::path::Path) -> Option<(Renderer, f64)> {
    let started = Instant::now();
    let options = RendererOptions { pipeline_cache_dir: Some(dir.to_path_buf()), ..test_support::options() };
    let mut r = match Renderer::headless((1920, 1080), options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("skipping: {e}");
            return None;
        }
    };
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    add_families(&mut r);
    let wanted = settings(r.capabilities());
    RenderBackend::apply_graphics(&mut r, &wanted);
    Some((r, started.elapsed().as_secs_f64() * 1000.0))
}

fn empty_dir() -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!("blackbox-prewarm-bench-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Time one prewarm with an empty pipeline cache and return (renderer and materials, prewarm) in ms.
fn measure() -> Option<(f64, f64)> {
    let dir = empty_dir();
    let (mut r, setup) = cold_renderer(&dir)?;
    let started = Instant::now();
    r.prewarm();
    let prewarm = started.elapsed().as_secs_f64() * 1000.0;
    drop(r);
    let _ = std::fs::remove_dir_all(&dir);
    Some((setup, prewarm))
}

/// Run it alone, once as is and once with `BLACKBOX_SERIAL_PIPELINES=1`, each in its own process: the driver keeps
/// what it compiled for the rest of the process.
#[test]
#[ignore = "needs a GPU; a benchmark, run alone with --nocapture"]
fn prewarm_bench_cold() {
    let _gpu = test_support::serial();
    let Some((setup, prewarm)) = measure() else { return };
    let mode = match std::env::var("BLACKBOX_SERIAL_PIPELINES").is_ok_and(|v| !v.is_empty() && v != "0") {
        true => "one pipeline at a time",
        false => "parallel",
    };
    eprintln!("cold pipeline cache, 1920x1080, high settings, every family, {mode}:");
    eprintln!("  renderer + materials {setup:.0} ms, prewarm {prewarm:.0} ms");
}
