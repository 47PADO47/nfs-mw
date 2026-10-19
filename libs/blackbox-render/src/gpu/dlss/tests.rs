//! Strict SDK/GPU smoke test: missing runtime/hardware is a failure, never a skipped pass.
use blackbox_gfx::{GraphicsSettings, RenderBackend, UpscaleQuality, Upscaler};
use blackbox_gfx_testkit::SceneId;

#[test]
#[ignore = "requires signed Streamline 2.14.16 runtime and an RTX Vulkan or DX12 adapter"]
fn dlss_sdk_evaluates_quality_modes_and_dlaa_on_gpu() {
    let _ =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).is_test(true).try_init();
    let _serial = super::super::test_support::serial();
    let (window, mut renderer) = super::test_window::TestWindow::renderer();
    assert!(renderer.capabilities().upscalers.contains(Upscaler::Dlss), "DLSS runtime must load on this RTX adapter");
    let built = SceneId::Grid.build(&mut renderer, [640, 360]);
    built.apply_layers(&mut renderer);
    for quality in [
        UpscaleQuality::Quality,
        UpscaleQuality::Balanced,
        UpscaleQuality::Performance,
        UpscaleQuality::UltraPerformance,
        UpscaleQuality::Native,
    ] {
        let settings = GraphicsSettings { upscaler: Upscaler::Dlss, upscale_quality: quality, ..Default::default() };
        assert!(renderer.apply_graphics(&settings).is_exact());
        assert_eq!(renderer.post_passes()[0], "dlss");
        let expected = super::render_size(quality, (640, 360)).expect("SDK optimal size");
        assert_eq!(renderer.render_size(), expected);
        if quality == UpscaleQuality::Native {
            assert_eq!(expected, (640, 360));
        }
        // SAFETY: the serial GPU lock excludes all other tests using this session.
        let before = unsafe { super::native::bb_sl_evaluations() };
        for _ in 0..8 {
            assert!(renderer.render(&built.frame, &built.instances).expect("render"));
        }
        renderer.device.poll(wgpu::PollType::wait_indefinitely()).expect("GPU completion");
        assert_eq!(unsafe { super::native::bb_sl_evaluations() }, before + 8, "every frame must evaluate real DLSS");
    }
    renderer.apply_graphics(&GraphicsSettings { upscaler: Upscaler::Fsr3, ..Default::default() });
    assert_eq!(renderer.post_passes()[0], "fsr3");
    renderer.render(&built.frame, &built.instances).expect("FSR 3 after DLSS");
    renderer.apply_graphics(&GraphicsSettings {
        upscaler: Upscaler::Dlss,
        upscale_quality: UpscaleQuality::Native,
        ..Default::default()
    });
    window.resize((800, 450));
    renderer.resize(800, 450);
    assert_eq!(renderer.render_size(), (800, 450));
    renderer.render(&built.frame, &built.instances).expect("DLAA after resize");
    let before = unsafe { super::native::bb_sl_evaluations() };
    let rendered_before = renderer.frame_performance().rendered_frames;
    renderer.capture(320, 180, &built.frame, &built.instances).expect("independent screenshot");
    assert_eq!(renderer.frame_performance().rendered_frames, rendered_before, "captures do not count as game frames");
    assert_eq!(unsafe { super::native::bb_sl_evaluations() }, before, "captures must not evaluate DLSS");
    renderer.render(&built.frame, &built.instances).expect("DLAA after capture");
    assert_eq!(unsafe { super::native::bb_sl_evaluations() }, before + 1);
    super::presentation_tests::exercise(&window, &mut renderer, &built);
    // Simulate a runtime loss: the next frame must resolve a truthful supported fallback.
    renderer.dlss.as_ref().unwrap().failed.store(true, std::sync::atomic::Ordering::Release);
    renderer.render(&built.frame, &built.instances).expect("fallback after runtime failure");
    assert_eq!(renderer.upscaler(), Upscaler::Fsr3);
    assert!(!renderer.capabilities().upscalers.contains(Upscaler::Dlss));
    built.release(&mut renderer);
    drop(renderer);
    drop(window);
    // Streamline plugin state cannot safely be initialized twice; a recreated renderer must fall back.
    let (_window, renderer) = super::test_window::TestWindow::renderer();
    assert!(!renderer.capabilities().upscalers.contains(Upscaler::Dlss));
}
