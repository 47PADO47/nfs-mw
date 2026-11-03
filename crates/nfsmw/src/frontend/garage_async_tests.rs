//! Installed worker publication, pending visibility and rollback; no real GPU submission.

use super::*;
use crate::app::test_backend::NullBackend;

fn settle(scene: &mut GarageScene, renderer: &mut NullBackend) {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !scene.ready() {
        assert!(std::time::Instant::now() < until, "car preview must finish");
        scene.update(renderer, &ActionState::default(), 1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn loaded() -> (GarageScene, NullBackend) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = GarageScene::new(dir, false);
    let mut renderer = NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_texture_tracking()
        .with_reflection_tracking();
    scene.init(&mut renderer).unwrap();
    settle(&mut scene, &mut renderer);
    assert!(scene.rendered.is_some(), "{:?}", scene.model.message);
    (scene, renderer)
}

#[test]
#[ignore = "requires installed My Cars assets"]
fn batch14_my_cars_unseen_previews_are_async_latest_safe_and_cached_on_return() {
    let (mut scene, mut renderer) = loaded();
    let original = scene.model.draft.clone();
    let choices: Vec<_> = scene
        .model
        .cars
        .iter()
        .filter(|car| car.folder != original.car)
        .take(2)
        .map(|car| Selection { car: car.folder.clone(), ..Default::default() })
        .collect();
    assert_eq!(choices.len(), 2);
    let meshes = renderer.mesh_labels.clone();
    scene.preview_garage(&mut renderer, choices[0].clone()).unwrap();
    assert!(scene.model.preview_loading && !scene.ready());
    assert_eq!(renderer.mesh_labels, meshes, "requesting a new preview does no immediate model upload");
    let (_, parts) = scene.frame(16.0 / 9.0);
    assert!(!parts.is_empty(), "the room remains visible while the car changes");
    assert!(
        parts.iter().all(|part| part.key.parts().0 & 0x3fff_ffff == (u32::MAX - 2) & 0x3fff_ffff),
        "the old car must not appear under a different selected name"
    );
    assert!(scene.confirm_garage(&mut renderer, choices[0].clone()).is_err(), "pending preview cannot be saved");
    scene.preview_garage(&mut renderer, choices[1].clone()).unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !scene.ready() {
        assert!(std::time::Instant::now() < until);
        scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
        assert_ne!(
            scene.rendered.as_ref().map(|selection| &selection.car),
            Some(&choices[0].car),
            "stale worker completion cannot become visible"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(scene.rendered.as_ref(), Some(&choices[1]));
    let after = renderer.mesh_labels.clone();
    scene.preview_garage(&mut renderer, original.clone()).unwrap();
    assert!(!scene.model.preview_loading);
    assert_eq!(scene.rendered.as_ref(), Some(&original));
    assert_eq!(renderer.mesh_labels, after, "returning to the cached car performs no upload");
    scene.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty() && renderer.texture_labels.is_empty());
}

#[test]
#[ignore = "requires installed My Cars assets"]
fn batch14_my_cars_async_error_rolls_back_and_close_prevents_late_publication() {
    let (mut scene, mut renderer) = loaded();
    let original = scene.model.draft.clone();
    scene.model.custom_slot = Some("original-slot".into());
    let meshes = renderer.mesh_labels.clone();
    let mut broken = scene.model.selected().unwrap().clone();
    broken.folder = "MISSING_ASYNC_TEST_CAR".into();
    broken.parts = None;
    scene.model.cars.push(broken);
    scene
        .preview_garage(&mut renderer, Selection { car: "MISSING_ASYNC_TEST_CAR".into(), ..Default::default() })
        .unwrap();
    settle(&mut scene, &mut renderer);
    assert!(scene.model.message.as_ref().unwrap().is_err());
    assert_eq!(scene.model.preview_update, crate::garage::PreviewUpdate::Failed);
    assert_eq!(scene.model.draft, original);
    assert_eq!(scene.model.custom_slot.as_deref(), Some("original-slot"));
    assert_eq!(renderer.mesh_labels, meshes);
    let other = scene
        .model
        .cars
        .iter()
        .find(|car| car.folder != original.car && car.folder != "MISSING_ASYNC_TEST_CAR")
        .unwrap()
        .folder
        .clone();
    scene.preview_garage(&mut renderer, Selection { car: other, ..Default::default() }).unwrap();
    scene.close(&mut renderer);
    scene.update(&mut renderer, &ActionState::default(), 1.0);
    assert!(scene.rendered.is_none());
    assert!(
        renderer.mesh_labels.is_empty() && renderer.texture_labels.is_empty(),
        "closed work cannot republish resources"
    );
}

/// Whether a frame draws any part other than the room.
fn shows_car(scene: &mut GarageScene) -> bool {
    let (_, parts) = scene.frame(16.0 / 9.0);
    parts.iter().any(|part| part.key.parts().0 & 0x3fff_ffff != (u32::MAX - 2) & 0x3fff_ffff)
}

#[test]
#[ignore = "requires installed My Cars assets"]
fn r2_same_car_edits_keep_the_car_while_only_a_car_change_empties_the_room() {
    let (mut scene, mut renderer) = loaded();
    let original = scene.model.draft.clone();
    let repainted = Selection { color: Some([200, 16, 16]), ..original.clone() };
    scene.preview_garage(&mut renderer, repainted.clone()).unwrap();
    assert!(scene.model.preview_loading, "a new paint needs a decode");
    assert!(shows_car(&mut scene), "the shown car stays while its own edit is prepared");
    settle(&mut scene, &mut renderer);
    assert_eq!(scene.rendered.as_ref(), Some(&repainted));
    let other = scene.model.cars.iter().find(|car| car.folder != original.car).unwrap().folder.clone();
    scene.preview_garage(&mut renderer, Selection { car: other, ..Default::default() }).unwrap();
    assert!(!shows_car(&mut scene), "a different car is never shown under the old car");
    scene.close(&mut renderer);
}

#[test]
fn batch14_my_cars_decode_identity_preserves_latest_session_fields_and_distinguishes_slots() {
    let first = decode::Request {
        selection: Selection { car: "BMWM3GTR".into(), ..Default::default() },
        slot: None,
        read_slot: false,
        commit: false,
        initial: false,
    };
    let mut next = first.clone();
    next.selection.gauges.needle = Some([12, 34, 56]);
    next.selection.tuning.profiles[0][0] = 2;
    next.commit = true;
    assert!(first.matches(&next), "a completed model can serve the latest non-geometry session edit");
    next.selection.color = Some([12, 34, 56]);
    assert!(!first.matches(&next), "different geometry cannot publish as the latest selection");
    let mut saved = first.clone();
    saved.slot = Some("saved-a".into());
    saved.read_slot = true;
    let mut second_slot = saved.clone();
    second_slot.slot = Some("saved-b".into());
    assert!(saved.matches(&saved));
    assert!(!saved.matches(&second_slot));
    assert!(!saved.matches(&first));
}
