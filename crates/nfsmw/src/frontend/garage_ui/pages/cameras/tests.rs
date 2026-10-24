use super::*;

#[test]
fn decal_camera_requests_keep_zone_context_for_each_position_and_room() {
    for backroom in [false, true] {
        for zone in 0..6 {
            let prefix = match backroom {
                false => "customize_decals",
                true => "customize_decals_backroom",
            };
            for position in 0..6 {
                let page = Page::Choices { row: Row::Decal(zone, position), backroom };
                assert_eq!(page.camera(), format!("{prefix}_{}", 1281 + zone));
                let ui = crate::frontend::garage_ui::GarageUi::new();
                ui.enter(page);
                assert!(ui.drain().into_iter().any(|request| matches!(request,
                    crate::frontend::garage_ui::Request::View(_, camera) if camera == page.camera())));
            }
            let prefix = match backroom {
                false => "customize_customizegenerictop",
                true => "customize_customizegenerictop_backroom",
            };
            assert_eq!(Page::DecalSlots { zone, backroom }.camera(), format!("{prefix}_{}", 1281 + zone));
        }
    }
}

#[test]
#[ignore = "requires installed original frontend camera data"]
fn installed_garage_decal_cameras_resolve_authored_location_poses() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let book = nfsmw_data::frontend_camera::PoseBook::load(&dir).unwrap();
    for backroom in [false, true] {
        let locations: Vec<_> = (0..6)
            .map(|zone| book.screen(Page::Choices { row: Row::Decal(zone, 0), backroom }.camera()).unwrap())
            .collect();
        assert_ne!(locations[0], locations[2], "windshield and left-door views must not share a generic fallback");
        assert_ne!(locations[2], locations[3], "the two doors have different authored views");
        assert_eq!(locations[2], book.angle("angle_decal__left_door").unwrap());
        assert_eq!(locations[3], book.angle("angle_decal_right_door").unwrap());
        let left = book.screen(Page::DecalSlots { zone: 2, backroom }.camera()).unwrap();
        let right = book.screen(Page::DecalSlots { zone: 3, backroom }.camera()).unwrap();
        let selector = match backroom {
            false => "angle_decal_doors",
            true => "angle_backroom_door_main",
        };
        assert_eq!(left, book.angle(selector).unwrap(), "the left-door selector uses its authored room overview");
        assert_eq!(right, book.angle(selector).unwrap(), "both door selectors deliberately share that overview");
    }
}
