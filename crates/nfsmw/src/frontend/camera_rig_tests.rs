//! A selected part camera must follow the stage intro, even without another selection event.
use super::*;

#[test]
#[ignore = "requires installed frontend camera collections"]
fn batch12_direct_part_camera_is_reached_after_the_stage_intro() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let book = PoseBook::load(&dir).unwrap();
    for screen in ["customize_customizeparts_260", "customize_customizeparts_261", "customize_rims"] {
        let rig = CameraRig::default();
        rig.begin(&dir, Stage::Customization, "customization_shop");
        rig.show(screen);
        let goal = book.screen(screen).expect("installed part view");
        assert_ne!(rig.pose().unwrap(), CameraPose::from_angle(&goal));
        for _ in 0..600 {
            rig.advance(1.0 / 60.0);
        }
        let pose = rig.pose().unwrap();
        assert!((pose.eye() - CameraPose::from_angle(&goal).eye()).length() < 0.001, "{screen}");
        assert_eq!(rig.screen_angle(), Some(goal));
    }
}

#[test]
#[ignore = "requires installed frontend camera collections"]
fn batch12_a_new_camera_request_replaces_the_deferred_intro_target() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let rig = CameraRig::default();
    rig.begin(&dir, Stage::Customization, "customization_shop");
    rig.show("customize_customizeparts_260");
    rig.show("customize_rims");
    let goal = PoseBook::load(&dir).unwrap().screen("customize_rims").unwrap();
    for _ in 0..600 {
        rig.advance(1.0 / 60.0);
    }
    assert!((rig.pose().unwrap().eye() - CameraPose::from_angle(&goal).eye()).length() < 0.001);
    rig.advance(20.0);
    assert!((rig.pose().unwrap().eye() - CameraPose::from_angle(&goal).eye()).length() < 0.001);
}
