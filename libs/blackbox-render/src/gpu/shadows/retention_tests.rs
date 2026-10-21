//! Clip-space UI views must not replace the world atlas or sample stale sun shadows.
use super::tests::scene;
use crate::gpu::test_support;
use blackbox_gfx::{InstanceKey, Projection, RenderBackend, RenderExperiments};

#[test]
#[ignore = "needs a GPU"]
fn clip_space_views_retain_atlas_without_stale_shadows_and_disable_releases_it() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless((160, 120)) else { return };
    let (frame, ground, mut casters) = scene(&mut r);
    casters[0].key = InstanceKey::new(1, 0);
    let ui = blackbox_gfx::FrameParams { projection: Projection::Identity, ..frame };
    let baseline = r.capture(160, 120, &ui, &ground).unwrap();
    r.set_shadow_casters(Some(&casters));
    for wide in [false, true] {
        let settings = RenderExperiments {
            sun_shadows: true,
            cached_shadows: true,
            shadow_resolution: 512,
            shadow_detail_resolution: 1024,
            wide_shadow_detail: wide,
            ..Default::default()
        };
        r.set_render_experiments(settings);
        r.capture(160, 120, &frame, &ground).unwrap();
        r.capture(160, 120, &frame, &ground).unwrap();
        let world = r.capture(160, 120, &frame, &ground).unwrap();
        let atlas = r.shared.shadows.view.clone();
        let detail = r.shared.shadows.detail.clone();
        let mib = r.experiment_stats().map_mib;
        let cached_mib = r.shadows.as_ref().unwrap().cache.mib();
        assert!(cached_mib > 0, "static cache must actually contain GPU maps");
        for _ in 0..3 {
            assert_eq!(r.capture(160, 120, &ui, &ground).unwrap(), baseline, "UI has no stale sun shadow");
            assert_eq!(r.shared.shadows.view, atlas, "same world atlas allocation");
            assert_eq!(r.shared.shadows.detail, detail, "same near-detail allocation");
            assert_eq!(r.experiment_stats().map_mib, mib, "retained memory stays visible in stats");
            assert_eq!(r.shadows.as_ref().unwrap().cache.mib(), cached_mib, "retain static depth images too");
            assert_eq!(r.capture(160, 120, &frame, &ground).unwrap(), world, "world shadow roundtrip");
            assert!(r.experiment_stats().shadow_cached_cascades > 0, "static depth survives UI views");
            assert_eq!(r.shared.shadows.view, atlas);
        }
        r.capture(160, 120, &ui, &ground).unwrap();
        casters[0].transform = glam::Mat4::from_translation(glam::Vec3::X * 2.0);
        r.set_shadow_casters(Some(&casters));
        let changed = r.capture(160, 120, &frame, &ground).unwrap();
        assert_ne!(world, changed, "movement during UI still updates the world shadow");
        r.set_render_experiments(RenderExperiments { cached_shadows: false, ..settings });
        assert_eq!(r.capture(160, 120, &frame, &ground).unwrap(), changed, "movement matches uncached redraw");
        casters[0].transform = glam::Mat4::IDENTITY;
        r.set_shadow_casters(Some(&casters));
        r.set_render_experiments(RenderExperiments { shadow_resolution: 1024, ..settings });
        r.capture(160, 120, &ui, &ground).unwrap();
        assert_eq!(r.shared.shadows.view, atlas, "defer resize until a shadow-capable view");
        r.capture(160, 120, &frame, &ground).unwrap();
        assert_eq!(r.shared.shadows.resolution, 1024);
        assert_ne!(r.shared.shadows.view, atlas, "apply new active resolution");
        r.set_render_experiments(Default::default());
        r.capture(160, 120, &ui, &ground).unwrap();
        assert_eq!(r.shared.shadows.resolution, 1, "explicit disable releases large maps");
    }
}
