//! Headless GPU checks of moving content: motion vectors reproject the history, and depth decides
//! what is a disocclusion, in every depth convention.

mod common;

use common::{
    all_finite, dump_ppm, psnr, resize_bilinear,
    scene::{Kind, Rig, SceneParams},
};
use fsr3_wgpu::{DebugTexture, DepthConvention, Fsr3Config, MotionVectorLayout, QualityMode};
use glam::{UVec2, Vec2};

const DISPLAY: UVec2 = UVec2::new(192, 108);
const BORDER: u32 = 16;
const FRAMES: u32 = 36;
/// The last frames of a run that are scored.
const TAIL: u32 = 6;
const VELOCITY: Vec2 = Vec2::new(1.5, 0.75);

fn config(depth: DepthConvention) -> Fsr3Config {
    Fsr3Config { output_format: wgpu::TextureFormat::Rgba32Float, depth, ..Fsr3Config::new() }
}

/// The scores of one run: PSNR against the supersampled scene of the last frames, and of a bilinear
/// upscale of the last frame's plain render.
struct Run {
    upscaled: Vec<f64>,
    bilinear: f64,
    /// Pixels the upscaler found disoccluded in its last frame.
    disoccluded: usize,
}

impl Run {
    fn mean(&self) -> f64 {
        self.upscaled.iter().sum::<f64>() / self.upscaled.len() as f64
    }
}

fn run(config: Fsr3Config, kind: Kind, velocity: Vec2, tweak: impl FnOnce(&mut Rig)) -> Run {
    let mut rig = Rig::new(config, QualityMode::Quality.render_size(DISPLAY), DISPLAY);
    tweak(&mut rig);
    let mut upscaled = Vec::new();
    let mut bilinear = 0.0;
    for frame in 0..FRAMES {
        let scene = SceneParams::new(kind).moving(velocity).at(frame);
        rig.frame(scene, frame == 0);
        if frame < FRAMES - TAIL {
            continue;
        }
        let reference = rig.reference_image(&scene);
        let image = rig.output_image();
        assert!(all_finite(&image), "frame {frame} is finite");
        upscaled.push(psnr(&image, &reference, DISPLAY, BORDER));
        if frame == FRAMES - 1 {
            let plain = rig.plain_render(&scene);
            bilinear = psnr(&resize_bilinear(&plain, rig.render, DISPLAY), &reference, DISPLAY, BORDER);
            dump_ppm(&format!("moving_{kind:?}_fsr3"), &image, DISPLAY);
            dump_ppm(&format!("moving_{kind:?}_reference"), &reference, DISPLAY);
        }
    }
    let (_, channels, masks) = rig.read_debug(DebugTexture::ReactiveMasks);
    let disoccluded = masks.chunks(channels).filter(|px| px[1] > 0.5).count();
    Run { upscaled, bilinear, disoccluded }
}

#[test]
#[ignore = "needs a GPU"]
fn a_translating_checkerboard_keeps_its_quality_with_correct_motion_vectors() {
    let depth = DepthConvention::REVERSE_INFINITE;
    let right = run(config(depth), Kind::Checker, VELOCITY, |_| {});
    let none = run(config(depth), Kind::Checker, VELOCITY, |rig| rig.motion_gain = 0.0);
    eprintln!(
        "checkerboard: correct vectors {:.2} dB ({:.2?}), zero vectors {:.2} dB, bilinear {:.2} dB",
        right.mean(),
        right.upscaled,
        none.mean(),
        right.bilinear
    );
    assert!(right.mean() > 19.0, "{:?}", right.upscaled);
    assert!(right.mean() > right.bilinear + 1.0, "better than a bilinear upscale of the last frame");
    assert!(right.mean() > none.mean() + 5.0, "the vectors matter: {} vs {}", right.mean(), none.mean());
}

#[test]
#[ignore = "needs a GPU"]
fn every_depth_convention_finds_the_disocclusions_behind_a_moving_object() {
    let velocity = Vec2::new(3.0, 1.0);
    let conventions = [
        DepthConvention::REVERSE_INFINITE,
        DepthConvention::REVERSE,
        DepthConvention::STANDARD,
        DepthConvention { inverted: false, infinite: true },
    ];
    let mut scores = Vec::new();
    for depth in conventions {
        let r = run(config(depth), Kind::Layers, velocity, |_| {});
        eprintln!("{depth:?}: {:.2} dB, {} disoccluded pixels", r.mean(), r.disoccluded);
        // The square is about 25 render pixels tall and moves 2 by 0.7 pixels per frame.
        assert!((50..130).contains(&r.disoccluded), "{depth:?}: {} pixels", r.disoccluded);
        assert!(r.mean() > r.bilinear + 2.0, "{depth:?}: {} vs {}", r.mean(), r.bilinear);
        scores.push(r.mean());
    }
    let spread = scores.iter().cloned().fold(f64::MIN, f64::max) - scores.iter().cloned().fold(f64::MAX, f64::min);
    assert!(spread < 1.0, "the conventions agree: {scores:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn a_wrong_depth_convention_misses_disocclusions() {
    // The buffer holds reverse infinite depth; declaring it standard turns the order of the layers
    // around, which hides the disocclusion.
    let velocity = Vec2::new(3.0, 1.0);
    let right = run(config(DepthConvention::REVERSE_INFINITE), Kind::Layers, velocity, |_| {});
    let wrong = run(config(DepthConvention::STANDARD), Kind::Layers, velocity, |rig| {
        rig.depth = DepthConvention::REVERSE_INFINITE;
    });
    eprintln!("disoccluded: right {}, wrong {}", right.disoccluded, wrong.disoccluded);
    assert!(wrong.disoccluded * 10 < right.disoccluded * 8, "{} vs {}", wrong.disoccluded, right.disoccluded);
}

#[test]
#[ignore = "needs a GPU"]
fn motion_vectors_at_the_output_resolution_work_like_render_resolution_ones() {
    let depth = DepthConvention::REVERSE_INFINITE;
    let render = run(config(depth), Kind::Checker, VELOCITY, |_| {});
    let display_config = Fsr3Config {
        motion_vectors: MotionVectorLayout { display_resolution: true, jittered: false },
        ..config(depth)
    };
    let display = run(display_config.clone(), Kind::Checker, VELOCITY, |_| {});
    let none = run(display_config, Kind::Checker, VELOCITY, |rig| rig.motion_gain = 0.0);
    eprintln!("render {:.2} dB, display {:.2} dB, zero {:.2} dB", render.mean(), display.mean(), none.mean());
    assert!(display.mean() > render.mean() - 1.5, "{} vs {}", display.mean(), render.mean());
    assert!(display.mean() > none.mean() + 5.0);
}

#[test]
#[ignore = "needs a GPU"]
fn jittered_motion_vectors_are_cancelled_when_the_layout_says_so() {
    let depth = DepthConvention::REVERSE_INFINITE;
    let plain = run(config(depth), Kind::Checker, VELOCITY, |_| {});
    let jittered = Fsr3Config {
        motion_vectors: MotionVectorLayout { display_resolution: false, jittered: true },
        ..config(depth)
    };
    let cancelled = run(jittered, Kind::Checker, VELOCITY, |rig| rig.jittered_motion_vectors = true);
    let uncancelled = run(config(depth), Kind::Checker, VELOCITY, |rig| rig.jittered_motion_vectors = true);
    eprintln!(
        "plain {:.2} dB, cancelled {:.2} dB, not cancelled {:.2} dB",
        plain.mean(),
        cancelled.mean(),
        uncancelled.mean()
    );
    assert!(cancelled.mean() > plain.mean() - 1.0, "{} vs {}", cancelled.mean(), plain.mean());
    assert!(cancelled.mean() > uncancelled.mean() + 1.0, "{} vs {}", cancelled.mean(), uncancelled.mean());
}
