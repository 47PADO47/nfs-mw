//! Read the actual fourth model slot and exercise its owning section's resource lifetime.
use crate::{app::test_backend::NullBackend, viewer::Scene};
use std::time::{Duration, Instant};

#[test]
#[ignore = "needs installed scenery selected by NFSMW_GAME_DIR"]
fn installed_road_reflection_slots_resolve_without_owning_extra_gpu_resources() {
    let (_, mut scene) = super::open();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    scene.init(&mut renderer).unwrap();
    let start = Instant::now();
    loop {
        scene.residency.update(&mut renderer, 333.0, 2517.0);
        if scene.residency.complete() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(60), "road geometry residency timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut reflected = 0;
    let mut fourth_slot = 0;
    let mut reflection_only = 0;
    let mut borrowed_meshes = std::collections::HashSet::new();
    for p in scene.residency.reflection_placed() {
        reflected += 1;
        reflection_only += usize::from(p.player_index.is_none());
        assert!(!p.key.is_transient(), "reflection sources must have stable section identities");
        for (handle, bounds) in p.lods.iter().zip(p.local_bounds) {
            let Some(handle) = handle else { continue };
            let bounds = bounds.expect("resident reflection mesh must retain its own bounds");
            assert!(!bounds.is_empty() && bounds.min.is_finite() && bounds.max.is_finite());
            borrowed_meshes.insert(*handle);
            assert!(renderer.mesh_labels.contains_key(handle), "reflected source points to a retired/missing mesh");
        }
        fourth_slot += usize::from(p.lods[3].is_some());
    }
    assert!(reflected > 0 && fourth_slot > 0, "real resident scenery must resolve authored slot3 models");
    eprintln!(
        "road reflection placements: {reflected}, resolved slot3: {fourth_slot}, reflection-only: {reflection_only}"
    );
    scene.close(&mut renderer);
    scene.close(&mut renderer);
    // WorldScene::close frees every resident tile, so the drain below finds nothing left; the reflected feed owns
    // no handles of its own.
    for state in scene.residency.tiles.drain().map(|(_, state)| state) {
        if let super::super::TileState::Resident { resources, .. } = state {
            resources.release(&mut renderer);
        }
    }
    std::mem::take(&mut scene.residency.shared).release(&mut renderer);
    assert!(
        borrowed_meshes.iter().all(|handle| !renderer.mesh_labels.contains_key(handle)),
        "every borrowed reflected mesh is released by its section owner"
    );
    assert!(renderer.mesh_labels.is_empty(), "section owners must release every mesh allocation");
    assert_eq!(renderer.resources.lock().unwrap().meshes, 0);
    assert!(renderer.projected.is_none());
}
