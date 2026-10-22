//! The front end's camera poses against the install (`frontend_camera`). Spec:
//! `docs/specs/menu-backdrop-looks.md` section 4.

use nfsmw_data::frontend_camera::{self, CameraAngle};

use crate::install;

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

fn is(angle: &CameraAngle, pose: [f32; 5], target: [f32; 3], shape: (f32, f32, i32)) -> bool {
    let [pitch, yaw, distance, roll, fov] = pose;
    near(angle.pitch, pitch)
        && near(angle.yaw, yaw)
        && near(angle.distance, distance)
        && near(angle.roll, roll)
        && near(angle.fov, fov)
        && angle.target.iter().zip(target).all(|(a, b)| near(*a, b))
        && near(angle.decay, shape.0)
        && near(angle.duration, shape.1)
        && angle.order == shape.2
}

/// `docs/specs/menu-backdrop-looks.md` section 4.3: the safe house move starts on the entry pose and runs 3.15 s onto
/// the final pose with the final pose's decay and order; the pose of the screen itself (`angle10`) has the same place but
/// a 0.6 s ease.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_safe_house_move_runs_from_its_entry_pose_to_its_final_pose() {
    let Some(dir) = install() else { return };
    let poses =
        frontend_camera::load(&dir, &["angle_entry_career_safehouse", "angle_final_career_safehouse", "angle10"])
            .expect("the safe house poses");
    assert!(is(&poses[0], [94.4, -58.0, 9.0, -13.1, 55.0], [2.3, -0.3, 1.0], (8.0, 0.6, 3)), "{:?}", poses[0]);
    assert!(is(&poses[1], [97.3, 110.0, 4.25, 16.0, 65.0], [-0.2, -0.5, 0.3], (9.0, 3.15, 1)), "{:?}", poses[1]);
    assert!(is(&poses[2], [97.3, 110.0, 4.25, 16.0, 65.0], [-0.2, -0.5, 0.3], (8.0, 0.6, 2)), "{:?}", poses[2]);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_missing_pose_is_an_error() {
    let Some(dir) = install() else { return };
    assert!(frontend_camera::load(&dir, &["angle_entry_nowhere"]).is_err());
}
