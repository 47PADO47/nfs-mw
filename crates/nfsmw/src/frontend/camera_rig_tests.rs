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

#[test]
#[ignore = "requires installed frontend camera collections"]
fn a_later_screen_during_the_intro_starts_its_own_move_at_once() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let book = PoseBook::load(&dir).unwrap();
    let duration = book.angle("angle_final_customization_shop").unwrap().duration;
    assert!(duration > 0.0);
    let rig = CameraRig::default();
    let control = CameraRig::default();
    for camera in [&rig, &control] {
        camera.begin(&dir, Stage::Customization, "customization_shop");
        camera.show("customize_customizeparts_260");
        camera.advance(duration * 0.2);
    }
    let from = rig.pose().unwrap();
    rig.show("customize_rims");
    assert_eq!(rig.pose(), Some(from), "the move starts where the intro has got to, without a jump");
    rig.advance(duration * 0.1);
    control.advance(duration * 0.1);
    assert!(
        (rig.pose().unwrap().eye() - control.pose().unwrap().eye()).length() > 0.001,
        "a second screen leaves the intro's path at once instead of waiting for it to finish",
    );
    rig.advance(20.0);
    let goal = CameraPose::from_angle(&book.screen("customize_rims").unwrap());
    assert!((rig.pose().unwrap().eye() - goal.eye()).length() < 0.001);
}
