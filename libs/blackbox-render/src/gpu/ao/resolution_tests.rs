//! The same contact geometry must keep its angular AO reach at higher scene resolutions.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{AoQuality, AoSettings, GraphicsSettings, RenderBackend, Shading};
use glam::Vec3;

#[test]
#[ignore = "needs a GPU"]
fn contact_reach_is_consistent_across_scene_resolutions() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless((1440, 1080)) else { return };
    let mut profiles = Vec::new();
    for size in [(1440, 1080), (2880, 2160)] {
        let (frame, instances) = scene_shading(&mut r, size, true, Shading::Prelit);
        r.apply_graphics(&GraphicsSettings {
            ao: AoSettings { quality: AoQuality::High, intensity: 1.0, ..Default::default() },
            ..Default::default()
        });
        r.capture(size.0, size.1, &frame, &instances).unwrap();
        let mask = r.shared.shadows.ambient.texture();
        let mask = r.read_raw_texture(mask, size, 1).unwrap();
        let matrix = frame.projection.matrix() * frame.view;
        let mut profile = Vec::new();
        for y in 0..10 {
            for x in 0..32 {
                let point = Vec3::new(-1.25 + x as f32 * 0.08, -0.82 - y as f32 * 0.06, 0.0);
                let p = matrix.project_point3(point);
                let px = ((p.x + 1.0) * 0.5 * size.0 as f32) as u32;
                let py = ((1.0 - p.y) * 0.5 * size.1 as f32) as u32;
                let radius = (size.1 / 360) as i32;
                let mut sum = 0.0;
                let mut count = 0;
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let ix = (px as i32 + dx).clamp(0, size.0 as i32 - 1) as u32;
                        let iy = (py as i32 + dy).clamp(0, size.1 as i32 - 1) as u32;
                        sum += f32::from(mask[(iy * size.0 + ix) as usize]);
                        count += 1;
                    }
                }
                profile.push(sum / count as f32);
            }
        }
        assert!(profile.iter().any(|&v| v < 230.0), "fixture must exercise visible ground occlusion");
        profiles.push(profile);
    }
    let error =
        profiles[0].iter().zip(&profiles[1]).map(|(a, b)| (a - b).abs()).sum::<f32>() / profiles[0].len() as f32;
    eprintln!("1080p/2160p ground contact mask mean difference: {error:.3}/255");
    assert!(error < 6.0, "AO reach must not shrink at higher scene resolution");
}
