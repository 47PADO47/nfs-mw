//! Distant traffic support uses real installed collision without uploading distant scenery.
use crate::{app::test_backend::NullBackend, viewer::Scene};
use std::time::{Duration, Instant};

#[test]
#[ignore = "needs installed world collision selected by NFSMW_GAME_DIR"]
fn installed_traffic_collision_prefetch_does_not_expand_colour_or_gpu_resources() {
    let (_, mut scene) = super::open();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    scene.init(&mut renderer).unwrap();
    let start = Instant::now();
    while !scene.residency.complete() {
        scene.residency.update(&mut renderer, 333.0, 2517.0);
        assert!(start.elapsed() < Duration::from_secs(60), "colour loading timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    let meshes = renderer.mesh_labels.len();
    let drawn = scene.residency.drawn.clone();
    scene.residency.set_traffic_support([2395.0, 2913.0], [2395.0, 2913.0], 900.0, vec![]);
    loop {
        scene.residency.update(&mut renderer, 333.0, 2517.0);
        assert!(scene.residency.complete(), "traffic support must not block ready scenery");
        let support = &scene.residency.traffic_support;
        let remaining = support.wanted.iter().any(|i| {
            !support.resident.contains_key(i) && !support.failed.contains(i) && !scene.residency.tiles.contains_key(i)
        });
        if !remaining && support.pending.is_empty() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "collision-only loading timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    let support = &scene.residency.traffic_support;
    assert!(support.failed.is_empty(), "collision requests must parse the actual section bytes");
    let packs: usize = support.resident.values().map(Vec::len).sum();
    assert!(packs > 0, "must load actual off-PVS collision, not merely empty sections");
    assert_eq!(renderer.mesh_labels.len(), meshes);
    assert_eq!(scene.residency.drawn, drawn);
    assert!(
        super::super::super::space::ground_below(scene.residency.collision(), 2395.0, 2913.0, 300.0, 0.0).is_some()
    );
    eprintln!("traffic collision: {packs} extra packs; {} sections; {meshes} unchanged meshes", support.resident.len());
    scene.close(&mut renderer);
}
