//! AO motion/depth must coexist with native and FSR jitter; strict SDK tests cover DLSS.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{
    AoMethod, AoQuality, AoSettings, GraphicsSettings, RenderBackend, Shading, UpscaleQuality, Upscaler,
};

#[test]
#[ignore = "needs a GPU"]
fn temporal_ao_contacts_survive_switching_upscalers_and_their_jitter() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let mut r = test_support::headless(size).expect("GPU adapter required");
    let (frame, mut instances) = scene_shading(&mut r, size, true, Shading::Prelit);
    instances[0].key = blackbox_gfx::InstanceKey::new(7, 2);
    assert!(r.capabilities().upscalers.contains(Upscaler::Fsr3));
    for upscaler in [Upscaler::Off, Upscaler::Fsr3, Upscaler::Off] {
        r.apply_graphics(&GraphicsSettings {
            upscaler,
            upscale_quality: UpscaleQuality::Native,
            ao: AoSettings { method: AoMethod::TemporalXeGtao, quality: AoQuality::Medium, ..Default::default() },
            ..Default::default()
        });
        assert_eq!(r.upscaler(), upscaler);
        for _ in 0..24 {
            r.render(&frame, &instances).unwrap();
        }
        let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
        assert!(mask.iter().filter(|&&v| v < 240).count() > 100, "{upscaler:?}: AO contacts were lost");
        assert!(mask[..size.0 as usize].iter().all(|&v| v >= 254), "{upscaler:?}: empty sky is not occluded");
        println!("{upscaler:?}: temporal AO contacts preserved");
    }
}
