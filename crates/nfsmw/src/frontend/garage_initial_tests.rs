//! Initial selection is prepared before the first upload, including invalid-slot fallback.

use super::*;
use crate::app::test_backend::NullBackend;

fn loaded(dir: &GameDir) -> (GarageScene, NullBackend) {
    let mut scene = GarageScene::new(dir.clone(), false);
    let mut renderer =
        NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_texture_tracking();
    scene.init(&mut renderer).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !scene.ready() {
        assert!(std::time::Instant::now() < deadline, "initial My Cars preview must finish");
        scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    (scene, renderer)
}

#[test]
#[ignore = "requires installed My Cars assets; writes only an isolated child-process profile"]
fn batch12_my_cars_initial_slot_uploads_once_and_invalid_slot_falls_back() {
    const CHILD: &str = "NFSMW_INITIAL_GARAGE_SLOT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let profile = std::env::temp_dir().join(format!("nfsmw-initial-slot-{}-{unique}", std::process::id()));
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "frontend::garage_scene::initial_tests::batch12_my_cars_initial_slot_uploads_once_and_invalid_slot_falls_back",
                "--exact",
                "--ignored",
            ])
            .env(CHILD, "1")
            .env("BLACKBOX_PROFILE_DIR", profile)
            .status()
            .unwrap();
        assert!(status.success(), "isolated initial-slot regression");
        return;
    }
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let (mut stock, mut renderer) = loaded(&dir);
    assert!(stock.model.message.is_none(), "{:?}", stock.model.message);
    assert!(stock.model.selected().unwrap().paintable);
    let saved = Selection { color: Some([25, 70, 180]), ..stock.model.draft.clone() };
    let stock_meshes = renderer.mesh_labels.len();
    stock.close(&mut renderer);
    let id = crate::garage::my_cars::create(&saved, "initial preview regression").unwrap();

    let (mut selected, mut renderer) = loaded(&dir);
    assert_eq!(selected.model.draft, saved);
    assert_eq!(selected.model.custom_slot.as_deref(), Some(id.as_str()));
    assert_eq!(renderer.mesh_labels.len(), stock_meshes, "entry must not park an unused stock preview");
    assert_eq!(renderer.resources.lock().unwrap().mesh_destructions, 0, "entry uploads only the selected car");
    selected.close(&mut renderer);

    crate::garage::my_cars::delete(&id).unwrap();
    crate::garage::my_cars::create(&Selection { car: "MISSING_TEST_CAR".into(), ..Default::default() }, "invalid")
        .unwrap();
    let (mut fallback, mut renderer) = loaded(&dir);
    assert!(fallback.model.message.as_ref().unwrap().as_ref().unwrap_err().contains("could not be loaded"));
    assert!(fallback.rendered.is_some());
    assert!(fallback.model.custom_slot.is_none());
    assert_ne!(fallback.model.draft.car, "MISSING_TEST_CAR");
    assert_eq!(renderer.mesh_labels.len(), stock_meshes);
    assert_eq!(renderer.resources.lock().unwrap().mesh_destructions, 0);
    fallback.begin_garage(&mut renderer).unwrap();
    fallback.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert!(renderer.texture_labels.is_empty());
}

#[test]
#[ignore = "requires installed My Cars assets"]
fn batch12_failed_garage_preview_preserves_saved_slot_draft_and_visible_car() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let (mut scene, mut renderer) = loaded(&dir);
    scene.model.custom_slot = Some("selected-saved-slot".into());
    let draft = scene.model.draft.clone();
    let rendered = scene.rendered.clone();
    let meshes = renderer.mesh_labels.clone();
    let textures = renderer.texture_labels.clone();
    let failed = Selection { car: "MISSING_TEST_CAR".into(), ..draft.clone() };
    assert!(scene.preview_garage(&mut renderer, failed).is_err());
    assert_eq!(scene.model.custom_slot.as_deref(), Some("selected-saved-slot"));
    assert_eq!(scene.model.draft, draft);
    assert_eq!(scene.rendered, rendered);
    assert_eq!(renderer.mesh_labels, meshes);
    assert_eq!(renderer.texture_labels, textures);
    scene.close(&mut renderer);
}
