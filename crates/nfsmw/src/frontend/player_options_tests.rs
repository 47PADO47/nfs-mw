//! Player category routing against installed native menu packages.

use super::*;
use crate::frontend::options::{Setting, Title, rows};
use crate::settings::CameraMode;
use blackbox_feng::{NodeKind, fe_hash_upper};

#[test]
fn camera_transmission_and_gauges_belong_to_player_in_native_order() {
    let caps = test_caps::native();
    let player = rows(Category::Player, &caps);
    assert_eq!(
        player.iter().map(|row| row.setting).collect::<Vec<_>>(),
        [Setting::Transmission, Setting::Camera, Setting::Hud]
    );
    assert_eq!(player[1].title, Title::Label(0xF6CC_DC5F));
    for category in [Category::Audio, Category::Video, Category::Gameplay, Category::Controls, Category::Discord] {
        assert!(
            !rows(category, &caps).iter().any(|row| matches!(row.setting, Setting::Transmission | Setting::Hud)),
            "{category:?}"
        );
    }
    for category in [Category::Audio, Category::Video, Category::Controls, Category::Discord] {
        assert!(!rows(category, &caps).iter().any(|row| row.setting == Setting::Camera), "{category:?}");
    }
}

#[test]
fn the_gameplay_page_also_switches_the_camera_and_keeps_it_in_the_settings() {
    let caps = test_caps::native();
    let gameplay = rows(Category::Gameplay, &caps);
    let camera = gameplay.iter().find(|row| row.setting == Setting::Camera).expect("a Camera row on Gameplay");
    assert_eq!(camera.title, Title::Label(0xF6CC_DC5F), "it reads as the Player page's row");
    let mut settings = crate::settings::Settings::from(crate::settings::Partial::default());
    let mut changed = crate::settings::Partial::default();
    assert_eq!(settings.camera_mode, CameraMode::Chase);
    Setting::Camera.step(&mut settings, &mut changed, true);
    assert_eq!(settings.camera_mode, CameraMode::DynamicFov);
    assert_eq!(changed.camera_mode, Some(CameraMode::DynamicFov), "the change is recorded, so it is saved");
    Setting::Camera.step(&mut settings, &mut changed, false);
    Setting::Camera.step(&mut settings, &mut changed, false);
    assert_eq!(settings.camera_mode, CameraMode::Pursuit, "backwards from the first camera wraps to the last");
}

#[test]
#[ignore = "requires installed menu packages and English strings; set NFSMW_GAME_DIR"]
fn player_camera_routes_from_normal_and_pause_options_and_requests_saving() {
    for pause in [false, true] {
        let (start, categories, options, steps) = match pause {
            false => (screen::MAIN_MENU, screen::MAIN_MENU_SUB, screen::OPTIONS, 7),
            true => (screen::PAUSE_MENU, screen::PAUSE_MENU, screen::PAUSE_OPTIONS, 1),
        };
        let mut h = Harness::open(start, Args { pause, ..Args::default() }).expect("installed menu packages");
        h.wait(1.0);
        for _ in 0..steps {
            h.press(pad::RIGHT);
        }
        h.press(pad::ACCEPT);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(categories));
        for _ in 0..3 {
            h.press(pad::RIGHT);
        }
        h.press(pad::ACCEPT);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(options));
        let tree = h.screens.trees().pop().unwrap();
        for (name, expected) in
            [("OPTION_NAME_1", "Transmission"), ("OPTION_NAME_2", "Camera"), ("OPTION_NAME_3", "Gauges")]
        {
            let node = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper(name)).unwrap();
            assert!(node.visible, "{options}: {name}");
            assert_eq!(node.text.as_deref(), Some(expected), "{options}: {name}");
        }
        let header = match pause {
            true => "PLAYER",
            false => "player",
        };
        assert!(tree.nodes.iter().any(|node| node.visible && node.text.as_deref() == Some(header)));
        // The native pause package has a category picture; main Options has only its title.
        if pause {
            assert!(tree.nodes.iter().any(|node| node.name_hash == 0x0800_7B4C
                && matches!(node.kind, NodeKind::Image { texture: 0xD708_EFEF, .. })));
        }
        assert!(
            !tree.nodes.iter().any(|node| node.visible && node.name_hash == fe_hash_upper("OPTION_NAME_4")),
            "unsupported original rows must remain hidden"
        );
        h.press(pad::DOWN);
        h.press(pad::RIGHT);
        assert_eq!(h.settings.camera_mode, CameraMode::DynamicFov);
        h.press(pad::LEFT);
        assert_eq!(h.settings.camera_mode, CameraMode::Chase);
        h.press(pad::RIGHT);
        assert_eq!(h.changed, Partial { camera_mode: Some(CameraMode::DynamicFov), ..Partial::default() });
        h.press(pad::BACK);
        h.wait(1.5);
        assert!(h.said(&Command::SaveSettings));
        assert_eq!(h.screens.top(), Some(categories));
        assert!(h.screens.trees().iter().any(|tree| {
            tree.nodes
                .iter()
                .any(|node| node.name_hash == super::super::ids::ICON_TITLE && node.text.as_deref() == Some("Player"))
        }));
    }
}
