//! Headless GPU checks of the context's state: reset, resizing, the optional masks, output formats and
//! inputs that could produce NaN or infinity.

mod common;

use common::{
    all_finite, psnr,
    scene::{Kind, Rig, SceneParams},
};
use fsr3_wgpu::{Fsr3Config, QualityMode};
use glam::{UVec2, Vec2};

const DISPLAY: UVec2 = UVec2::new(192, 108);
const BORDER: u32 = 8;

fn config() -> Fsr3Config {
    Fsr3Config { output_format: wgpu::TextureFormat::Rgba32Float, ..Fsr3Config::new() }
}

fn rig_with(config: Fsr3Config, mode: QualityMode) -> Rig {
    Rig::new(config, mode.render_size(DISPLAY), DISPLAY)
}

fn rig() -> Rig {
    rig_with(config(), QualityMode::Quality)
}

const JITTER: Vec2 = Vec2::new(0.25, -0.125);

#[test]
#[ignore = "needs a GPU"]
fn reset_drops_the_history() {
    let before = SceneParams::new(Kind::Static);
    let after = SceneParams::new(Kind::Inverted);

    // A context that has never seen the scene before.
    let mut fresh = rig();
    fresh.frame_with_jitter(after, JITTER, false);
    let fresh_image = fresh.output_image();

    let run = |reset: bool| {
        let mut rig = rig();
        for frame in 0..24 {
            rig.frame(before, frame == 0);
        }
        rig.frame_with_jitter(after, JITTER, reset);
        rig.output_image()
    };
    let reset = psnr(&run(true), &fresh_image, DISPLAY, 0);
    let kept = psnr(&run(false), &fresh_image, DISPLAY, 0);
    eprintln!("after a scene change: with reset {reset:.1} dB from a fresh context, without {kept:.1} dB");
    assert!(reset > 60.0, "a reset frame equals the first frame of a new context: {reset}");
    assert!(kept < reset - 10.0, "without a reset the old image still shows: {kept}");
}

#[test]
#[ignore = "needs a GPU"]
fn the_context_reset_method_does_the_same_as_the_flag() {
    let before = SceneParams::new(Kind::Static);
    let after = SceneParams::new(Kind::Inverted);
    let mut a = rig();
    let mut b = rig();
    for frame in 0..12 {
        a.frame(before, frame == 0);
        b.frame(before, frame == 0);
    }
    a.frame_with_jitter(after, JITTER, true);
    b.ctx.reset();
    b.frame_with_jitter(after, JITTER, false);
    assert!(psnr(&a.output_image(), &b.output_image(), DISPLAY, 0) > 60.0);
}

#[test]
#[ignore = "needs a GPU"]
fn a_new_size_recreates_the_images_and_starts_over() {
    let scene = SceneParams::new(Kind::Smooth);
    let mut rig = rig();
    for frame in 0..16 {
        rig.frame(scene, frame == 0);
    }
    let small = rig.ctx.gpu_memory_bytes();
    assert!(small > 0);

    // The same display with another quality mode: the render size changes.
    let render = QualityMode::Performance.render_size(DISPLAY);
    rig.resize(render, DISPLAY);
    rig.frame(scene, false);
    let performance = rig.ctx.gpu_memory_bytes();
    assert!(performance < small, "a smaller render size needs less memory: {performance} < {small}");
    for frame in 0..24 {
        rig.frame(scene, false);
        assert!(all_finite(&rig.output_image()), "frame {frame} after the resize is finite");
    }
    let converged = psnr(&rig.output_image(), &rig.reference_image(&scene), DISPLAY, BORDER);

    // A bigger output.
    let big = DISPLAY * 2;
    rig.resize(QualityMode::Quality.render_size(big), big);
    for _ in 0..24 {
        rig.frame(scene, false);
    }
    assert!(rig.ctx.gpu_memory_bytes() > small);
    let image = rig.output_image();
    assert_eq!(image.len(), (big.x * big.y) as usize);
    let upscaled = psnr(&image, &rig.reference_image(&scene), big, BORDER * 2);
    eprintln!("after resizing: performance {converged:.2} dB, 2x output {upscaled:.2} dB");
    assert!(converged > 24.0 && upscaled > 24.0);
}

#[test]
#[ignore = "needs a GPU"]
fn a_reactive_mask_makes_the_upscaler_trust_the_current_frame() {
    let scene = SceneParams::new(Kind::Smooth);
    let score = |tweak: &dyn Fn(&mut Rig)| {
        let mut rig = rig();
        tweak(&mut rig);
        for frame in 0..40 {
            rig.frame(scene, frame == 0);
        }
        let image = rig.output_image();
        assert!(all_finite(&image));
        psnr(&image, &rig.reference_image(&scene), DISPLAY, BORDER)
    };
    let none = score(&|_| {});
    let reactive = score(&|rig| rig.reactive = Some(1.0));
    let transparent = score(&|rig| rig.transparency = Some(1.0));
    eprintln!("no mask {none:.2} dB, reactive {reactive:.2} dB, transparency {transparent:.2} dB");
    // With everything reactive the history cannot sharpen the picture.
    assert!(reactive < none - 1.5, "{reactive} vs {none}");
    // The transparency mask only feeds the reactive channel, which loosens the history clamp.
    assert!(transparent < none, "{transparent} vs {none}");
    let zero = score(&|rig| rig.reactive = Some(0.0));
    assert!((zero - none).abs() < 0.2, "an empty mask changes nothing: {zero} vs {none}");
}

#[test]
#[ignore = "needs a GPU"]
fn the_output_formats_agree() {
    let scene = SceneParams::new(Kind::Smooth);
    let render = |format: wgpu::TextureFormat, read: &dyn Fn(&Rig) -> Vec<[f32; 4]>| {
        let mut rig = rig_with(Fsr3Config { output_format: format, ..config() }, QualityMode::Quality);
        rig.sharpness = Some(0.8);
        for frame in 0..24 {
            rig.frame(scene, frame == 0);
        }
        read(&rig)
    };
    let f32_image = render(wgpu::TextureFormat::Rgba32Float, &Rig::output_image);
    let f16_image = render(wgpu::TextureFormat::Rgba16Float, &Rig::output_image_f16);
    let u8_image = render(wgpu::TextureFormat::Rgba8Unorm, &Rig::output_image_u8);
    assert!(all_finite(&f32_image) && all_finite(&f16_image));
    let half = psnr(&f16_image, &f32_image, DISPLAY, 0);
    let byte = psnr(&u8_image, &f32_image, DISPLAY, 0);
    eprintln!("16-bit float {half:.1} dB, 8-bit {byte:.1} dB from the 32-bit float output");
    assert!(half > 55.0 && byte > 40.0);
}

#[test]
#[ignore = "needs a GPU"]
fn low_dynamic_range_input_works_too() {
    let scene = SceneParams::new(Kind::Smooth);
    let score = |hdr: bool| {
        let mut rig = rig_with(Fsr3Config { hdr, ..config() }, QualityMode::Quality);
        for frame in 0..40 {
            rig.frame(scene, frame == 0);
        }
        psnr(&rig.output_image(), &rig.reference_image(&scene), DISPLAY, BORDER)
    };
    let (hdr, ldr) = (score(true), score(false));
    eprintln!("hdr path {hdr:.2} dB, ldr path {ldr:.2} dB");
    assert!(ldr > 30.0 && (hdr - ldr).abs() < 2.5);
}

#[test]
#[ignore = "needs a GPU"]
fn extreme_inputs_never_produce_nan_or_infinity() {
    for auto_exposure in [false, true] {
        for gain in [0.0, 1e-30, 1e-5, 1.0, 1e4, 6.0e4, 3.0e7] {
            for sharpness in [None, Some(1.0)] {
                let mut rig = rig_with(Fsr3Config { auto_exposure, ..config() }, QualityMode::Balanced);
                rig.sharpness = sharpness;
                for frame in 0..10 {
                    // Alternating kinds make the shading change and the locks fire as well.
                    let kind = if frame % 4 == 3 { Kind::Inverted } else { Kind::Static };
                    rig.frame(SceneParams::new(kind).gain(gain), frame == 0);
                    let image = rig.output_image();
                    assert!(
                        all_finite(&image),
                        "auto exposure {auto_exposure}, gain {gain}, sharpness {sharpness:?}, frame {frame}"
                    );
                    assert!(image.iter().all(|t| t[..3].iter().all(|v| *v >= 0.0)), "gain {gain}: no negative light");
                }
            }
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn a_reset_every_frame_and_extreme_jitter_stay_finite() {
    let mut rig = rig_with(config(), QualityMode::UltraPerformance);
    for frame in 0..8 {
        let jitter = Vec2::new(if frame % 2 == 0 { -0.5 } else { 0.5 }, if frame % 3 == 0 { -0.5 } else { 0.5 });
        rig.frame_with_jitter(SceneParams::new(Kind::Static), jitter, true);
        assert!(all_finite(&rig.output_image()));
    }
    // A delta time of 0 and a very long one, with auto exposure on.
    let mut rig = rig_with(Fsr3Config { auto_exposure: true, ..config() }, QualityMode::Quality);
    for frame in 0..6 {
        rig.frame(SceneParams::new(Kind::Smooth).gain(20.0), frame == 0);
        assert!(all_finite(&rig.output_image()));
    }
}
