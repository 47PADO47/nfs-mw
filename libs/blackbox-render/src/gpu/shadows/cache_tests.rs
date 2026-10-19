//! Pixel equality, live movement, removal, sun changes and resize invalidate cached depth.
use super::tests::scene;
use crate::gpu::test_support;
use blackbox_gfx::{InstanceKey, RenderBackend, RenderExperiments};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn cached_static_depth_matches_redraw_and_dynamic_changes_do_not_trail() {
    let _gpu = test_support::serial();
    let size = (240, 160);
    let mut r = test_support::headless(size).expect("GPU required");
    let (mut frame, ground, mut casters) = scene(&mut r);
    casters[0].key = InstanceKey::new(1, 0);
    let original = RenderExperiments { sun_shadows: true, shadow_resolution: 512, ..Default::default() };
    let cached = RenderExperiments { cached_shadows: true, ..original };
    r.set_render_experiments(original);
    r.set_shadow_casters(Some(&casters));
    let reference = r.capture(size.0, size.1, &frame, &ground).unwrap();
    r.set_render_experiments(cached);
    for _ in 0..4 {
        assert_eq!(reference, r.capture(size.0, size.1, &frame, &ground).unwrap());
    }
    assert!(r.experiment_stats().shadow_cached_cascades > 0, "must actually reuse static depth");
    assert_eq!(r.experiment_stats().shadow_draws, 0, "unchanged static scene needs no raster draws");
    let mut moving = casters[0];
    moving.key = InstanceKey::TRANSIENT;
    moving.transform = Mat4::from_translation(Vec3::X * -2.5);
    casters.push(moving);
    r.set_shadow_casters(Some(&casters));
    let mixed = r.capture(size.0, size.1, &frame, &ground).unwrap();
    r.set_render_experiments(original);
    assert_eq!(mixed, r.capture(size.0, size.1, &frame, &ground).unwrap());
    r.set_render_experiments(cached);
    for _ in 0..4 {
        assert_eq!(mixed, r.capture(size.0, size.1, &frame, &ground).unwrap());
    }
    assert!(r.experiment_stats().shadow_cached_cascades > 0);
    assert!(r.experiment_stats().shadow_draws > 0, "unkeyed dynamic caster still redraws");
    for x in [2.0, 3.0, 3.0, 3.0] {
        casters[0].transform = Mat4::from_translation(Vec3::X * x);
        r.set_shadow_casters(Some(&casters));
        let reused = r.capture(size.0, size.1, &frame, &ground).unwrap();
        r.set_render_experiments(original);
        assert_eq!(reused, r.capture(size.0, size.1, &frame, &ground).unwrap(), "moving caster must match fresh depth");
        r.set_render_experiments(cached);
    }
    r.set_shadow_casters(Some(&[]));
    let removed = r.capture(size.0, size.1, &frame, &ground).unwrap();
    r.set_render_experiments(original);
    assert_eq!(removed, r.capture(size.0, size.1, &frame, &ground).unwrap());
    r.set_shadow_casters(Some(&casters));
    r.set_render_experiments(cached);
    frame.light_dir = Vec3::new(-0.4, 0.3, -1.0).normalize();
    let sun = r.capture(size.0, size.1, &frame, &ground).unwrap();
    r.set_render_experiments(original);
    assert_eq!(sun, r.capture(size.0, size.1, &frame, &ground).unwrap());
    r.resize(127, 93);
    r.set_render_experiments(cached);
    r.capture(127, 93, &frame, &ground).unwrap();
    r.set_render_experiments(Default::default());
    r.capture(127, 93, &frame, &ground).unwrap();
    assert_eq!(r.shadows.as_ref().unwrap().cache.mib(), 0, "disabled shadows release cached maps");
}

#[test]
fn transient_and_duplicate_keys_never_enter_the_static_cache() {
    use blackbox_gfx::{Instance, MeshHandle};
    let mut cache = super::cache::Cache::default();
    let mesh = MeshHandle::from_raw(0);
    let keyed = Instance::keyed(mesh, Mat4::IDENTITY, InstanceKey::new(1, 0));
    let transient = Instance::new(mesh, Mat4::IDENTITY);
    cache.prepare(&[keyed, transient]);
    cache.prepare(&[keyed, transient]);
    assert_eq!(cache.stable, [true, false]);
    cache.prepare(&[keyed, keyed]);
    assert_eq!(cache.stable, [false, false]);
    cache.prepare(&[keyed]);
    assert_eq!(cache.stable, [false], "an ambiguous prior key cannot establish history");
}
