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
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn native_options_scrollbar_is_hidden_when_rows_fit_and_kept_for_overflow() {
    const SCROLL_GROUP: u32 = 0xA808_E057;
    let mut player = Harness::open(screen::OPTIONS, Args { category: Category::Player, ..Args::default() })
        .expect("installed menu packages");
    assert!(rows(Category::Player, &test_caps::native()).len() <= 9);
    let check_hidden = |h: &Harness| {
        let tree = h.screens.trees().pop().unwrap();
        let group = tree.nodes.iter().find(|node| node.name_hash == SCROLL_GROUP).expect("main scrollbar group");
        let mut inside = vec![group.index];
        loop {
            let before = inside.len();
            for node in &tree.nodes {
                if node.parent.is_some_and(|parent| inside.contains(&parent)) && !inside.contains(&node.index) {
                    inside.push(node.index);
                }
            }
            if inside.len() == before {
                break;
            }
        }
        assert!(inside.len() > 2, "the group must include the scrollbar children");
        assert!(
            inside.iter().all(|index| !tree.nodes[*index].visible && !tree.draw_order.contains(index)),
            "scrollbar nodes: {:?}",
            inside.iter().map(|i| (tree.nodes[*i].name_hash, tree.nodes[*i].visible)).collect::<Vec<_>>()
        );
    };
    check_hidden(&player);
    player.wait(1.5);
    check_hidden(&player);
    let tree = player.screens.trees().pop().unwrap();
    assert!(tree.nodes.iter().any(|n| n.visible && n.text.as_deref() == Some("Camera")));
    player.press(pad::DOWN);
    let shown = player.screens.presented_trees(&crate::input::Bindings::default(), crate::input::InputDevice::Xbox);
    assert!(shown.last().unwrap().nodes.iter().any(|n| n.visible && n.text.as_deref() == Some("Done")));
    player.run(pad::BACK, 1);
    for _ in 0..60 {
        if player.screens.top() != Some(screen::OPTIONS) {
            break;
        }
        check_hidden(&player);
        player.run(0, 1);
    }
    assert_eq!(player.screens.top(), Some(screen::MAIN_MENU_SUB));

    let mut video = Harness::open(screen::OPTIONS, Args { category: Category::Video, ..Args::default() })
        .expect("installed menu packages");
    assert!(rows(Category::Video, &test_caps::native()).len() > 9, "control must overflow the main window");
    video.wait(1.5);
    let tree = video.screens.trees().pop().unwrap();
    let group = tree.nodes.iter().find(|node| node.name_hash == SCROLL_GROUP).expect("main scrollbar group");
    assert!(group.visible && group.world_colour[3] > 0, "overflow keeps the native scrollbar group");
    let first = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper("OPTION_NAME_1")).unwrap().text.clone();
    for _ in 0..9 {
        video.press(pad::DOWN);
    }
    let tree = video.screens.trees().pop().unwrap();
    let now = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper("OPTION_NAME_1")).unwrap().text.clone();
    assert_ne!(first, now, "navigation scrolls the overflowing row window");
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
        // Keep Back held across the transition: the new screen must wait for neutral input.
        h.run(pad::BACK, 180);
        assert_eq!(h.screens.top(), Some(categories), "held Back must not leave the returned categories");
        h.run(0, 3);
        h.wait(1.5);
        assert!(h.said(&Command::SaveSettings));
        assert_eq!(h.screens.top(), Some(categories));
        assert!(h.screens.trees().iter().any(|tree| {
            tree.nodes
                .iter()
                .any(|node| node.name_hash == super::super::ids::ICON_TITLE && node.text.as_deref() == Some("Player"))
        }));
        assert_eq!(h.settings.camera_mode, CameraMode::DynamicFov);
        h.press(pad::ACCEPT);
        h.wait(1.5);
        assert_eq!(h.screens.top(), Some(options));
        let tree = h.screens.trees().pop().unwrap();
        let camera = tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper("OPTION_NAME_2")).unwrap();
        assert!(camera.visible);
        assert_eq!(camera.text.as_deref(), Some("Camera"), "Accept must reopen Player, not Audio");
        assert_eq!(h.settings.camera_mode, CameraMode::DynamicFov, "Camera survives the round trip");
    }
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn video_scrollbar_arrows_are_visible_and_move_selection_without_changing_settings() {
    let mut h = Harness::open(screen::OPTIONS, Args { category: Category::Video, ..Args::default() })
        .expect("installed menu packages");
    h.wait(1.5);
    let tree = h.screens.trees().pop().unwrap();
    let root = tree.nodes.iter().find(|n| n.name_hash == 0xA808_E057).unwrap().index;
    let arrow = |hash| {
        let group = tree.nodes.iter().find(|n| n.name_hash == hash && n.parent == Some(root)).unwrap();
        let image = tree
            .nodes
            .iter()
            .find(|n| n.parent == Some(group.index) && tree.draw_order.contains(&n.index) && n.world_colour[3] > 0)
            .expect("visible arrow image");
        image.world.transform_point3(glam::Vec3::ZERO).truncate()
    };
    let cursor_y =
        |tree: &blackbox_feng::UiTree| tree.nodes.iter().find(|n| n.name_hash == 0x0674_5352).unwrap().world.w_axis.y;
    let first_y = cursor_y(&tree);
    let up = arrow(0x4449_69FD);
    let down = arrow(0x4449_69FE);
    let before = h.settings;
    assert!(h.click(down));
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    assert!(tree.nodes.iter().any(|n| n.name_hash == fe_hash_upper("RIGHT_ARROW_2") && n.visible));
    assert_ne!(cursor_y(&tree), first_y, "down moves the selection cursor");
    assert_eq!(h.settings, before);
    assert!(h.click(up));
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    assert!(tree.nodes.iter().any(|n| n.name_hash == fe_hash_upper("RIGHT_ARROW_1") && n.visible));
    assert!((cursor_y(&tree) - first_y).abs() < 0.01, "up restores the cursor");
    assert_eq!(h.settings, before);
    assert!(!tree.nodes.iter().any(|n| n.name_hash == fe_hash_upper("RIGHT_ARROW_2") && n.visible));
    let count = rows(Category::Video, &test_caps::native()).len();
    for _ in 0..count - 1 {
        assert!(h.click(down));
        h.wait(0.1);
    }
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    assert!(tree.nodes.iter().any(|n| n.name_hash == fe_hash_upper("RIGHT_ARROW_9") && n.visible), "last row");
    assert!(h.click(down));
    h.wait(0.5);
    let tree = h.screens.trees().pop().unwrap();
    assert!(tree.nodes.iter().any(|n| n.name_hash == fe_hash_upper("RIGHT_ARROW_1") && n.visible), "wrap to first");
    assert!(!tree.nodes.iter().any(|n| n.name_hash == fe_hash_upper("RIGHT_ARROW_9") && n.visible));
    assert_eq!(h.settings, before);
    h.run(pad::BACK, 1);
    let previous = h.settings;
    assert!(!h.click(down), "leaving rejects the arrow click");
    assert_eq!(h.settings, previous, "leaving cannot change settings");

    let mut player = Harness::open(screen::OPTIONS, Args { category: Category::Player, ..Args::default() }).unwrap();
    player.wait(1.5);
    let tree = player.screens.trees().pop().unwrap();
    let group = tree.nodes.iter().find(|n| n.name_hash == 0xA808_E057).unwrap();
    assert!(!group.visible, "short list hides the scrollbar parent");
    let before = player.settings;
    assert!(!player.click(down), "short list rejects the arrow click");
    assert_eq!(player.settings, before);
    let mut pause =
        Harness::open(screen::PAUSE_OPTIONS, Args { pause: true, category: Category::Video, ..Args::default() })
            .unwrap();
    pause.wait(1.5);
    assert!(!pause.screens.trees().pop().unwrap().nodes.iter().any(|n| n.name_hash == 0xA808_E057));
}
