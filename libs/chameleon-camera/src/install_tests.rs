//! The cameras of the 2012 install's cars. Set `CHAMELEON_GAME_DIR` to the install folder and run with `--ignored`.

use std::path::{Path, PathBuf};

use chameleon_bundle::Bundle;

use crate::{CameraOptions, CameraState, CameraView, CarCameras, CarMotion, read_all_cameras, read_cameras};

/// `VEHICLES/VEH_<n>_HI.BNDL` of the install, sorted.
fn car_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join("VEHICLES"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy().to_uppercase();
            name.starts_with("VEH_") && name.ends_with("_HI.BNDL")
        })
        .collect();
    files.sort();
    files
}

fn all_cars(dir: &Path) -> Vec<(String, CarCameras)> {
    car_files(dir)
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let data = std::fs::read(&path).unwrap();
            let bundle = Bundle::parse(&data).unwrap();
            let cameras = read_cameras(&bundle).unwrap_or_else(|error| panic!("{name}: {error}"));
            (name, cameras)
        })
        .collect()
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_car_has_the_three_cameras_with_plausible_values() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let cars = all_cars(Path::new(&dir));
    assert_eq!(cars.len(), 93, "the 2012 install has 93 cars");
    for (name, cameras) in &cars {
        let ext = &cameras.external;
        // behind (x < 0) by 4 to 6.1 m and 1.4 to 3 m up, looking at a point 0.5 to 1.3 m ahead and 1 to 1.7 m up
        assert!((-6.2..=-3.9).contains(&ext.position[0]), "{name}: {:?}", ext.position);
        assert!((1.4..=3.1).contains(&ext.position[2]) && ext.position[1] == 0.0, "{name}: {:?}", ext.position);
        assert!((0.5..=1.3).contains(&ext.focus[0]) && (1.0..=1.7).contains(&ext.focus[2]), "{name}: {:?}", ext.focus);
        assert_eq!(ext.fov_range_deg(), [70.0, 95.0], "{name}");
        assert!((1.3..=1.8).contains(&ext.height_at_speed), "{name}: {}", ext.height_at_speed);
        assert!((-1.3..=0.0).contains(&ext.speed_movement.back_at_speed), "{name}");
        assert!((120.0..=210.0).contains(&ext.speed_movement.shake.full_speed_mph), "{name}");
        assert!((2.0..=6.0).contains(&ext.roll.max_deg) && (25.0..=45.0).contains(&ext.roll.full_speed_mph), "{name}");
        assert!((2.0..=6.0).contains(&ext.decel_pitch.max_deg), "{name}");
        assert!((0.04..=0.08).contains(&ext.lag_s), "{name}");
        assert_eq!(ext.yaw_spring.rate_low, 10.0, "{name}");
        let (bonnet, bumper) = (&cameras.bonnet, &cameras.bumper);
        assert!(bonnet.eye[2] > 0.5 && bonnet.eye[2] < 2.0 && bonnet.eye[0].abs() < 1.0, "{name}: {:?}", bonnet.eye);
        // the SWAT van (1467242) is the one car whose bonnet and bumper differ from the rest: its bumper holds the
        // numbers every other car has on its bonnet
        let van = name.contains("1467242");
        assert_eq!(bonnet.fov_range_deg(), if van { [100.0, 100.0] } else { [95.0, 105.0] }, "{name}");
        assert_eq!(bumper.fov_range_deg(), [95.0, 120.0], "{name}");
        assert!(bumper.eye.iter().all(|v| v.abs() < 3.0) && bumper.target.iter().all(|v| v.abs() < 3.0), "{name}");
        if !van {
            assert!(bumper.eye[2] > 0.3 && bumper.eye[2] < 1.0 && bumper.eye[0] < 0.0, "{name}: {:?}", bumper.eye);
            assert!(bumper.target[0] > 1.0 && bumper.target[0] < 2.5, "{name}: {:?}", bumper.target);
        }
        assert_eq!(cameras.rear_view.fov_deg, 60.0, "{name}");
        assert_eq!(cameras.globals.lens.fov_deg, 90.0, "{name}");
        assert_eq!(cameras.gradient.pitch_follow, [0.5, 0.5], "{name}");
        assert_eq!(cameras.globals.impact_shake.frequency, [10.0, 10.0, 50.0], "{name}");
    }
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_globals_and_the_rear_view_are_shared_by_all_cars() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let cars = all_cars(Path::new(&dir));
    let first = &cars[0].1;
    for (name, cameras) in &cars {
        assert_eq!(cameras.globals, first.globals, "{name}");
        assert_eq!(cameras.rear_view.offset, first.rear_view.offset, "{name}");
        assert_eq!(cameras.rear_view.aspect, first.rear_view.aspect, "{name}");
        assert_eq!(cameras.external.yaw_spring, first.external.yaw_spring, "{name}");
        assert_eq!(cameras.external.response, first.external.response, "{name}");
    }
    let differing = cars.iter().filter(|(_, cameras)| cameras.external.position != first.external.position).count();
    assert!(differing > 10, "the chase position is per car: {differing} cars differ from the first");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_porsche_911_has_the_documented_values() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let path = Path::new(&dir).join("VEHICLES/VEH_1085007_HI.BNDL");
    let data = std::fs::read(path).unwrap();
    let cameras = read_cameras(&Bundle::parse(&data).unwrap()).unwrap();
    assert_eq!(cameras.external.position, [-4.6, 0.0, 1.6]);
    assert_eq!(cameras.external.focus, [0.75, 0.0, 1.2]);
    assert_eq!(cameras.external.height_at_speed, 1.59);
    assert_eq!(cameras.external.speed_movement.shake.full_speed_mph, 180.0);
    assert_eq!(cameras.bumper.eye, [-1.5, 0.0, 0.6]);
    assert_eq!(cameras.bumper.target, [1.5, 0.0, 0.425]);
    assert_eq!(cameras.globals.lens.fov_deg, 90.0);
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_traffic_bundle_has_eleven_containers() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let data = std::fs::read(Path::new(&dir).join("VEHICLES/TRAFFICATTRIBS.BNDL")).unwrap();
    let all = read_all_cameras(&Bundle::parse(&data).unwrap()).unwrap();
    assert_eq!(all.len(), 11);
    for (id, cameras) in &all {
        assert!(cameras.external.position[0] < -3.0, "{id:016X}");
    }
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_model_drives_every_car_without_a_bad_number() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    for (name, cameras) in all_cars(Path::new(&dir)) {
        for view in CameraView::ALL {
            let mut state = CameraState::new(&cameras, &CameraOptions::default());
            state.set_view(view);
            let mut car = CarMotion::at_rest();
            let mut pose = state.reset(&car);
            for frame in 0..300 {
                car.speed = (frame as f32 * 0.2).min(60.0);
                car.velocity = [car.speed, 0.0, 0.0];
                car.position[0] += car.speed / 60.0;
                car.steer = (frame as f32 * 0.03).sin();
                pose = state.step(&car);
            }
            let finite = [pose.position, pose.forward, pose.up].iter().flatten().all(|v| v.is_finite());
            assert!(
                finite && pose.vertical_fov_deg > 20.0 && pose.vertical_fov_deg < 120.0,
                "{name} {view:?}: {pose:?}"
            );
        }
    }
}
