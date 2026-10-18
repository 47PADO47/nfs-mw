//! Runs within the strict SDK test's sole process session, rather than initializing another device.
use blackbox_gfx::{GraphicsSettings, LatencySettings, ReflexMode, RenderBackend, UpscaleQuality, Upscaler};

pub(super) fn exercise(
    window: &super::test_window::TestWindow,
    renderer: &mut crate::Renderer,
    built: &blackbox_gfx_testkit::scenes::BuiltScene,
) {
    let status = renderer.latency_status();
    eprintln!("Streamline presentation: {status:?}");
    assert!(status.reflex_available, "strict test requires real Reflex support");
    assert!(status.max_generated_frames > 0, "strict test requires DLSS-G support, including HAGS");
    window.resize((1280, 720));
    renderer.resize(1280, 720);
    renderer.set_vsync(false);
    window.visible();
    renderer
        .set_display_settings(blackbox_gfx::DisplaySettings { hdr: blackbox_gfx::HdrMode::Off, ..Default::default() });
    renderer.apply_graphics(&GraphicsSettings {
        upscaler: Upscaler::Dlss,
        upscale_quality: UpscaleQuality::Native,
        ..Default::default()
    });
    for reflex in ReflexMode::ALL {
        let before = unsafe { super::presentation::bb_sl_markers(0) };
        for _ in 0..12 {
            window.pump();
            assert!(renderer.begin_latency_frame(LatencySettings {
                reflex,
                frame_limit_us: 16_667,
                gameplay: true,
                ..Default::default()
            }));
            renderer.render(&built.frame, &built.instances).expect("Reflex frame");
        }
        assert_eq!(unsafe { super::presentation::bb_sl_markers(0) }, before + 12);
        for marker in 1..=5 {
            assert_eq!(
                unsafe { super::presentation::bb_sl_markers(marker) },
                unsafe { super::presentation::bb_sl_markers(0) },
                "simulation, submission and present markers must use the same frame cadence"
            );
        }
    }
    for generated in 1..=status.max_generated_frames.min(5) {
        let mut saw_generated_present = false;
        for index in 0..32 {
            window.pump();
            renderer.begin_latency_frame(LatencySettings {
                reflex: ReflexMode::Off,
                generated_frames: generated,
                frame_limit_us: 16_667,
                gameplay: true,
            });
            let mut frame = built.frame;
            frame.view = glam::Mat4::from_translation(glam::Vec3::new(index as f32 * 0.002, 0.0, 0.0)) * frame.view;
            frame.camera_position = frame.view.inverse().w_axis.truncate();
            renderer.render(&frame, &built.instances).expect("frame generation");
            let state = renderer.latency_status();
            if !saw_generated_present && state.presented_frames > 1 {
                eprintln!("DLSS-G {generated}: {state:?}");
            }
            assert!(state.generation_active, "{state:?}");
            saw_generated_present |= state.presented_frames == generated + 1;
        }
        assert!(saw_generated_present, "SDK must actually present generated frames, not merely accept options");
    }
    renderer.set_display_settings(blackbox_gfx::DisplaySettings::default());
    if renderer.display_status().active {
        renderer.begin_latency_frame(LatencySettings { generated_frames: 1, gameplay: true, ..Default::default() });
        renderer.render(&built.frame, &built.instances).expect("scRGB generation suspension");
        assert!(!renderer.latency_status().generation_active, "SDK does not support FP16/scRGB generation");
    }
    renderer
        .set_display_settings(blackbox_gfx::DisplaySettings { hdr: blackbox_gfx::HdrMode::Off, ..Default::default() });
    renderer.begin_latency_frame(LatencySettings { generated_frames: 1, gameplay: false, ..Default::default() });
    renderer.render(&built.frame, &built.instances).expect("menu suspension");
    assert!(!renderer.latency_status().generation_active);
    renderer.set_vsync(true);
    renderer.begin_latency_frame(LatencySettings { generated_frames: 1, gameplay: true, ..Default::default() });
    renderer.render(&built.frame, &built.instances).expect("VSync fallback");
    assert!(!renderer.latency_status().generation_active);
    renderer.set_vsync(false);
    renderer.resize(800, 450);
}
