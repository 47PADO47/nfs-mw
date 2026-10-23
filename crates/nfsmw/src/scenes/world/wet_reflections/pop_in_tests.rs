//! Installed camera traversal: retain sources across the original large cutoff.
use super::super::{Options, WorldScene, visibility};
use crate::{app::test_backend::NullBackend, input::ActionState, viewer::Scene};
use blackbox_gfx::InstanceKey;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires installed highway geometry"]
fn installed_enhanced_reflections_retain_original_cutoff_crossings() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    let mut world = WorldScene::open(
        &dir,
        Options {
            at: Some([-130.0, 4089.0]),
            height: 3.0,
            heading: 20.0,
            pitch: -12.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: None,
        },
    )
    .unwrap();
    world.init(&mut renderer).unwrap();
    let start = Instant::now();
    while !world.ready() {
        world.update(&mut renderer, &ActionState::default(), 0.0);
        assert!(start.elapsed() < Duration::from_secs(60));
        std::thread::sleep(Duration::from_millis(5));
    }
    let initial = world.gameplay_lens(16.0 / 9.0);
    let mut previous: Option<(HashSet<InstanceKey>, HashSet<InstanceKey>)> = None;
    let mut retained = 0;
    let mut original_changes = 0;
    let mut counts = Vec::new();
    for step in 0..61 {
        world.camera.position = initial.position
            + glam::Vec3::new(initial.forward.x, initial.forward.y, 0.0).normalize_or_zero() * (step as f32 * 2.0);
        let lens = world.gameplay_lens(16.0 / 9.0);
        let camera = visibility::Camera {
            view_proj: lens.view_proj(),
            position: lens.position.to_array(),
            forward: lens.forward.to_array(),
            fov_y_radians: lens.fov_y_degrees.to_radians(),
            light_dir: [0.0; 3],
        };
        let collect = |height| {
            let mut out = Vec::new();
            super::geometry::scenery(
                world.residency.reflection_placed(),
                &camera,
                world.residency.props(),
                height,
                &mut out,
            );
            out.into_iter().map(|i| i.key).collect::<HashSet<_>>()
        };
        let (original, enhanced) = (collect(240.0), collect(960.0));
        if let Some((before, stable)) = &previous {
            for key in original.symmetric_difference(before) {
                original_changes += 1;
                retained += usize::from(enhanced.contains(key) && stable.contains(key));
            }
        }
        counts.push((original.len(), enhanced.len()));
        previous = Some((original, enhanced));
    }
    eprintln!(
        "120m static-residency traversal: {original_changes} original membership changes; {retained} retained across both enhanced frames; counts {counts:?}"
    );
    assert!(retained > 0, "actual source pop-in must remain drawn across an enhanced transition");
    world.close(&mut renderer);
    assert_eq!(renderer.resources.lock().unwrap().meshes, 0);
}
