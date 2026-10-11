//! Headless GPU checks of a static scene: the upscaler runs within the default limits, converges
//! towards a supersampled reference and handles exposure.

mod common;

use common::{
    all_finite, dump_ppm, psnr, resize_bilinear,
    scene::{Kind, Rig, SceneParams},
};
use fsr3_wgpu::{Fsr3Config, QualityMode};
use glam::UVec2;

const DISPLAY: UVec2 = UVec2::new(192, 108);
const BORDER: u32 = 8;

fn config() -> Fsr3Config {
    Fsr3Config { output_format: wgpu::TextureFormat::Rgba32Float, ..Fsr3Config::new() }
}

fn rig(mode: QualityMode) -> Rig {
    Rig::new(config(), mode.render_size(DISPLAY), DISPLAY)
}

fn mean_luma(image: &[[f32; 4]]) -> f64 {
    image.iter().map(|t| f64::from(t[0] + t[1] + t[2]) / 3.0).sum::<f64>() / image.len() as f64
}

/// The PSNR of the output against the reference after each of the listed frame counts, and of a bilinear
/// upscale of the plain render.
fn converge(mode: QualityMode, kind: Kind, checkpoints: &[u32]) -> (Vec<f64>, f64) {
    let mut rig = rig(mode);
    let scene = SceneParams::new(kind);
    let reference = rig.reference_image(&scene);
    let plain = rig.plain_render(&scene);
    let baseline = psnr(&resize_bilinear(&plain, rig.render, DISPLAY), &reference, DISPLAY, BORDER);
    let mut curve = Vec::new();
    for frame in 1..=*checkpoints.last().unwrap() {
        rig.frame(scene, frame == 1);
        if checkpoints.contains(&frame) {
            let image = rig.output_image();
            assert!(all_finite(&image), "{mode:?} frame {frame} is finite");
            curve.push(psnr(&image, &reference, DISPLAY, BORDER));
        }
    }
    let name = format!("static_{kind:?}_{mode:?}");
    dump_ppm(&format!("{name}_reference"), &reference, DISPLAY);
    dump_ppm(&format!("{name}_bilinear"), &resize_bilinear(&plain, rig.render, DISPLAY), DISPLAY);
    dump_ppm(&format!("{name}_fsr3"), &rig.output_image(), DISPLAY);
    (curve, baseline)
}

#[test]
#[ignore = "needs a GPU"]
fn a_static_scene_converges_towards_the_supersampled_reference() {
    // Content with detail finer than the render resolution but no edge sharper than 3 pixels, so that
    // the jittered samples can reconstruct it.
    for (mode, margin) in [(QualityMode::Quality, 2.5), (QualityMode::Performance, 2.5)] {
        let (curve, baseline) = converge(mode, Kind::Smooth, &[1, 4, 16, 64]);
        eprintln!("{mode:?}: bilinear {baseline:.2} dB, upscaler at 1, 4, 16, 64 frames {curve:.2?}");
        assert!(curve[3] > baseline + margin, "{mode:?}: {curve:?} against {baseline} dB for a bilinear upscale");
        assert!(curve[3] > curve[0] + 3.0, "{mode:?}: more frames help: {curve:?}");
        assert!(curve[3] > curve[1] && curve[2] > curve[1], "{mode:?}: improves steadily: {curve:?}");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn native_resolution_anti_aliases_hard_edges() {
    // At 1x the upscaler is temporal anti-aliasing: the jittered frames average the edges.
    let (curve, baseline) = converge(QualityMode::NativeAa, Kind::Smooth, &[1, 8, 32]);
    eprintln!("NativeAa: no anti-aliasing {baseline:.2} dB, upscaler {curve:.2?}");
    assert!(curve[1] > baseline + 3.0, "{curve:?} against {baseline} dB without anti-aliasing");
}

#[test]
#[ignore = "needs a GPU"]
fn every_quality_mode_runs_and_stays_finite_on_hard_content() {
    for mode in QualityMode::ALL {
        let (curve, baseline) = converge(mode, Kind::Static, &[2, 24]);
        eprintln!("{mode:?}: bilinear {baseline:.2} dB, upscaler {curve:.2?}");
        assert!(curve[1] > 17.0, "{mode:?} stays close to the scene: {curve:?}");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn sharpening_adds_contrast_and_stays_close_to_the_scene() {
    let scene = SceneParams::new(Kind::Smooth);
    let run = |sharpness: Option<f32>| {
        let mut rig = rig(QualityMode::Quality);
        rig.sharpness = sharpness;
        for frame in 0..24 {
            rig.frame(scene, frame == 0);
        }
        (rig.output_image(), rig.reference_image(&scene))
    };
    let (plain, reference) = run(None);
    let (sharp, _) = run(Some(1.0));
    let (soft, _) = run(Some(0.0));
    let energy = |image: &[[f32; 4]]| -> f64 {
        let w = DISPLAY.x as usize;
        let mut sum = 0.0;
        for y in 1..DISPLAY.y as usize - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                let laplacian =
                    4.0 * image[i][0] - image[i - 1][0] - image[i + 1][0] - image[i - w][0] - image[i + w][0];
                sum += f64::from(laplacian.abs());
            }
        }
        sum
    };
    eprintln!("laplacian energy: none {:.1}, soft {:.1}, sharp {:.1}", energy(&plain), energy(&soft), energy(&sharp));
    assert!(all_finite(&sharp) && all_finite(&soft));
    assert!(energy(&sharp) > energy(&plain) * 1.02, "full sharpening adds contrast");
    assert!(energy(&sharp) > energy(&soft), "sharpness scales the effect");
    assert!(psnr(&sharp, &reference, DISPLAY, BORDER) > 30.0, "sharpening does not wreck the image");
    assert!(psnr(&sharp, &plain, DISPLAY, BORDER) > 30.0);
}

#[test]
#[ignore = "needs a GPU"]
fn auto_exposure_keeps_the_brightness_of_hdr_content() {
    for gain in [0.05, 1.0, 40.0] {
        let scene = SceneParams::new(Kind::Smooth).gain(gain);
        let mut rig = Rig::new(
            Fsr3Config { auto_exposure: true, ..config() },
            QualityMode::Quality.render_size(DISPLAY),
            DISPLAY,
        );
        for frame in 0..40 {
            rig.frame(scene, frame == 0);
        }
        let image = rig.output_image();
        let reference = rig.reference_image(&scene);
        assert!(all_finite(&image));
        let ratio = mean_luma(&image) / mean_luma(&reference);
        eprintln!("gain {gain}: output/reference brightness {ratio:.4}");
        assert!((ratio - 1.0).abs() < 0.03, "gain {gain}: {ratio}");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn an_exposure_input_scales_the_work_but_not_the_result() {
    let scene = SceneParams::new(Kind::Smooth);
    let mut results = Vec::new();
    for exposure in [None, Some(0.25), Some(3.0)] {
        let mut rig = rig(QualityMode::Quality);
        rig.exposure = exposure;
        for frame in 0..24 {
            rig.frame(scene, frame == 0);
        }
        let image = rig.output_image();
        assert!(all_finite(&image));
        results.push((exposure, image, rig.reference_image(&scene)));
    }
    let base = psnr(&results[0].1, &results[0].2, DISPLAY, BORDER);
    for (exposure, image, reference) in &results[1..] {
        let score = psnr(image, reference, DISPLAY, BORDER);
        eprintln!("exposure {exposure:?}: {score:.2} dB (default {base:.2})");
        assert!((score - base).abs() < 3.0, "{exposure:?}: {score} vs {base}");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn a_pre_exposure_change_is_compensated_in_the_history() {
    // The application doubles the exposure it renders with: the colour and the reported pre-exposure
    // double together. Told about it, the upscaler keeps its history; not told, it shows the old half.
    let run = |report: bool| {
        let mut rig = rig(QualityMode::Quality);
        let mut score = 0.0;
        for frame in 0..28u32 {
            let (gain, pre) = if frame < 24 { (1.0, 1.0) } else { (2.0, 2.0) };
            rig.pre_exposure = if report { pre } else { 1.0 };
            let scene = SceneParams::new(Kind::Smooth).gain(gain);
            rig.frame(scene, frame == 0);
            if frame == 24 {
                score = psnr(&rig.output_image(), &rig.reference_image(&scene), DISPLAY, BORDER);
            }
        }
        score
    };
    let told = run(true);
    let not_told = run(false);
    eprintln!("first frame after the change: told {told:.2} dB, not told {not_told:.2} dB");
    assert!(told > not_told + 3.0, "{told} vs {not_told}");
}
