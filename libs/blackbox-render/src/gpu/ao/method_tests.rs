//! Estimators share the same output contract and switch live without retaining the other method.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{AoMethod, AoQuality, AoSettings, GraphicsSettings, RenderBackend, Shading};

#[test]
#[ignore = "needs a GPU"]
fn estimators_have_contacts_open_planes_and_deterministic_live_switching() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let Some(mut r) = test_support::headless(size) else { return };
    let (frame, closed) = scene_shading(&mut r, size, true, Shading::Prelit);
    let (_, open) = scene_shading(&mut r, size, false, Shading::Prelit);
    for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High] {
        for depth_mips in [false, true] {
            let mut images = Vec::new();
            for method in [AoMethod::Gtao, AoMethod::Ssao, AoMethod::XeGtao, AoMethod::Gtao] {
                r.apply_graphics(&GraphicsSettings {
                    ao: AoSettings { method, quality, depth_mips, intensity: 1.0, ..Default::default() },
                    ..Default::default()
                });
                r.capture(size.0, size.1, &frame, &open).unwrap();
                let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
                assert!(
                    mask.iter().all(|&v| v >= 254),
                    "{method}/{quality} depth_mips={depth_mips}: open plane self occluded, min={:?}",
                    mask.iter().min()
                );
                let image = r.capture(size.0, size.1, &frame, &closed).unwrap();
                let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
                assert!(mask.iter().filter(|&&v| v < 245).count() > 100, "{method}/{quality}: contacts missing");
                assert_eq!(image, r.capture(size.0, size.1, &frame, &closed).unwrap(), "same frame changed");
                images.push(image);
            }
            assert_eq!(images[0], images[3], "AO method round trip retained stale data");
            assert_ne!(images[0], images[1], "estimators must exercise different kernels");
            assert_ne!(images[0], images[2], "XeGTAO must exercise its own kernel and denoiser");
        }
    }
}
