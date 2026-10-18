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
    use blackbox_gfx::{UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};
    let texture = UiTextureId::from_raw(7);
    renderer.update_ui_texture(&UiTexturePatch { id: texture, offset: None, size: [1, 1], rgba: &[255; 4] });
    renderer.set_ui_layer(UiLayer {
        pixels_per_point: 1.0,
        meshes: [[255; 4], [64, 64, 64, 64], [32, 16, 0, 0]]
            .into_iter()
            .enumerate()
            .map(|(i, color_rgba)| {
                let x = i as f32 * 64.0;
                UiMesh {
                    vertices: [[x, 0.0], [x + 64.0, 0.0], [x + 64.0, 32.0], [x, 32.0]]
                        .into_iter()
                        .map(|position| UiVertex { position, uv: [0.5; 2], color_rgba })
                        .collect(),
                    indices: vec![0, 1, 2, 0, 2, 3],
                    texture,
                    clip: [0.0, 0.0, 1280.0, 720.0],
                }
            })
            .collect(),
    });
    for hdr in [blackbox_gfx::HdrMode::Off, blackbox_gfx::HdrMode::Auto] {
        renderer.set_display_settings(blackbox_gfx::DisplaySettings { hdr, ..Default::default() });
        eprintln!("Presentation display: {:?}", renderer.display_status());
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
    }
    if renderer.display_status().active {
        assert_eq!(renderer.output.format(), wgpu::TextureFormat::Rgb10a2Unorm);
        assert_eq!(renderer.capture(32, 18, &built.frame, &built.instances).unwrap().len(), 32 * 18 * 4);
        assert_eq!(renderer.capture_hdr_linear(32, 18, &built.frame, &built.instances).unwrap().len(), 32 * 18);
    }
    renderer.begin_latency_frame(LatencySettings { generated_frames: 1, gameplay: false, ..Default::default() });
    renderer.render(&built.frame, &built.instances).expect("menu suspension");
    assert!(!renderer.latency_status().generation_active);
    renderer.set_vsync(true);
    renderer.begin_latency_frame(LatencySettings { generated_frames: 1, gameplay: true, ..Default::default() });
    renderer.render(&built.frame, &built.instances).expect("VSync fallback");
    assert!(!renderer.latency_status().generation_active);
    renderer.set_vsync(false);
    renderer.resize(800, 450);
    renderer.begin_latency_frame(LatencySettings::default());
    renderer.render(&built.frame, &built.instances).expect("generation off restores scRGB");
    if renderer.display_status().active {
        assert_eq!(renderer.output.format(), wgpu::TextureFormat::Rgba16Float);
    }
    renderer.set_ui_layer(UiLayer::default());
    renderer.free_ui_texture(texture);
}
