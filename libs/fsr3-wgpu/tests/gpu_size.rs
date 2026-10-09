//! A headless GPU check at a real resolution: full HD output, with odd render sizes, and the time a
//! frame takes on the device at hand (for information).

mod common;

use std::time::Instant;

use common::{
    all_finite, psnr,
    scene::{Kind, Rig, SceneParams},
};
use fsr3_wgpu::{Fsr3Config, QualityMode};
use glam::UVec2;

#[test]
#[ignore = "needs a GPU"]
fn full_hd_output_runs_in_every_quality_mode() {
    let display = UVec2::new(1920, 1080);
    for mode in [QualityMode::Quality, QualityMode::Balanced, QualityMode::Performance] {
        let config = Fsr3Config { output_format: wgpu::TextureFormat::Rgba16Float, ..Fsr3Config::new() };
        let mut rig = Rig::new(config, mode.render_size(display), display);
        rig.sharpness = Some(0.8);
        let scene = SceneParams::new(Kind::Smooth);
        let mut start = Instant::now();
        for frame in 0..30 {
            if frame == 10 {
                // Wait for the warm-up frames so the timing below covers 20 whole frames.
                rig.gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                start = Instant::now();
            }
            rig.frame(scene, frame == 0);
        }
        rig.gpu.device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let per_frame = start.elapsed().as_secs_f64() * 1000.0 / 20.0;
        let image = rig.output_image_f16();
        assert!(all_finite(&image));
        let score = psnr(&image, &rig.reference_image(&scene), display, 16);
        eprintln!(
            "{mode:?} {}x{} -> 1920x1080 on {}: {per_frame:.2} ms per frame including the scene, {:.2} dB, {:.1} MiB",
            rig.render.x,
            rig.render.y,
            rig.gpu.name,
            score,
            rig.ctx.gpu_memory_bytes() as f64 / (1024.0 * 1024.0)
        );
        assert!(score > 28.0, "{mode:?}: {score} dB");
        assert!(per_frame < 1000.0, "a frame takes {per_frame} ms");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn odd_sizes_work() {
    // Sizes that are not multiples of the workgroup, of 2 or of anything else.
    for (render, display) in [
        (UVec2::new(97, 53), UVec2::new(163, 91)),
        (UVec2::new(3, 2), UVec2::new(7, 5)),
        (UVec2::new(2, 2), UVec2::new(2, 2)),
        (UVec2::new(129, 65), UVec2::new(129, 65)),
    ] {
        let config =
            Fsr3Config { output_format: wgpu::TextureFormat::Rgba32Float, auto_exposure: true, ..Fsr3Config::new() };
        let mut rig = Rig::new(config, render, display);
        rig.sharpness = Some(0.5);
        for frame in 0..12 {
            rig.frame(SceneParams::new(Kind::Static), frame == 0);
            let image = rig.output_image();
            assert_eq!(image.len(), (display.x * display.y) as usize);
            assert!(all_finite(&image), "{render} -> {display}, frame {frame}");
        }
    }
}
