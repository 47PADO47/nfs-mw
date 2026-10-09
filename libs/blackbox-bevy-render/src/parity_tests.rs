//! GPU tests: the testkit scenes drawn by the Bevy renderer, headless, against the native renderer.
//!
//! Needs a GPU (`cargo test -p blackbox-bevy-render -- --include-ignored`); skips, not fails, when the machine
//! has no adapter. `BLACKBOX_GPU_FALLBACK=1` uses the software adapter, `BLACKBOX_DUMP_DIR=<dir>` writes the
//! pictures (native, Bevy, and the amplified difference) for looking at. The tolerances are the plan's.

use std::path::Path;

use blackbox_gfx::{GraphicsApi, RenderBackend, RgbaImage};
use blackbox_gfx_testkit::{ImageDiff, Mask, SceneId, capture_scene, compare};
use blackbox_gpu_passes::test_support::serial;
use blackbox_render::{Renderer, RendererOptions};

use crate::HeadlessBevy;

const SIZE: [u32; 2] = [320, 180];

fn fallback() -> bool {
    std::env::var("BLACKBOX_GPU_FALLBACK").is_ok_and(|v| !v.is_empty() && v != "0")
}

fn native(size: [u32; 2]) -> Option<Renderer> {
    let options = RendererOptions {
        backend: GraphicsApi::Vulkan,
        vsync: false,
        force_fallback_adapter: fallback(),
    };
    match Renderer::headless((size[0], size[1]), options) {
        Ok(renderer) => Some(renderer),
        Err(e) => {
            eprintln!("skipping: {e}");
            None
        }
    }
}

fn bevy(size: [u32; 2]) -> Option<HeadlessBevy> {
    match HeadlessBevy::new(size, GraphicsApi::Vulkan, fallback()) {
        Ok(renderer) => Some(renderer),
        Err(e) => {
            eprintln!("skipping: {e}");
            None
        }
    }
}

fn write_png(path: &Path, image: &RgbaImage) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
}

/// Write the pair and an amplified difference into `BLACKBOX_DUMP_DIR`, when it is set.
fn dump(name: &str, native: &RgbaImage, bevy: &RgbaImage) {
    let Ok(dir) = std::env::var("BLACKBOX_DUMP_DIR") else { return };
    let dir = Path::new(&dir);
    write_png(&dir.join(format!("{name}_native.png")), native);
    write_png(&dir.join(format!("{name}_bevy.png")), bevy);
    let mut diff = native.clone();
    for (d, (a, b)) in diff.rgba.chunks_mut(4).zip(native.rgba.chunks(4).zip(bevy.rgba.chunks(4))) {
        for c in 0..3 {
            d[c] = a[c].abs_diff(b[c]).saturating_mul(16);
        }
        d[3] = 255;
    }
    write_png(&dir.join(format!("{name}_diff.png")), &diff);
}

/// Both renderers' pictures of `scene` and how far apart they are.
fn compare_scene(scene: SceneId, size: [u32; 2]) -> Option<ImageDiff> {
    let mut native = native(size)?;
    let a = capture_scene(&mut native, scene, size).expect("native capture");
    drop(native);
    let mut bevy = bevy(size)?;
    let b = capture_scene(&mut bevy, scene, size).expect("bevy capture");
    dump(scene.name(), &a, &b);
    Some(compare(&a, &b, &Mask::all()).expect("same size"))
}

fn check(scene: SceneId) {
    let _gpu = serial();
    let Some(diff) = compare_scene(scene, SIZE) else { return };
    let tolerance = scene.fidelity().tolerance();
    eprintln!("{}: {diff} (tolerance max {}, mean {}, p99 {})", scene.name(), tolerance.max, tolerance.mean, tolerance.p99);
    tolerance.check(&diff).unwrap_or_else(|e| panic!("{}: {e}", scene.name()));
}

#[test]
#[ignore = "needs a GPU"]
fn the_grid_matches_native() {
    check(SceneId::Grid);
}

#[test]
#[ignore = "needs a GPU"]
fn alpha_tested_cards_match_native() {
    check(SceneId::AlphaCards);
}

#[test]
#[ignore = "needs a GPU"]
fn the_sky_dome_matches_native() {
    check(SceneId::SkyDome);
}

#[test]
#[ignore = "needs a GPU"]
fn the_depth_probe_matches_native() {
    check(SceneId::DepthProbe);
}
