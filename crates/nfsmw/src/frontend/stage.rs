//! The stage a backdrop look stands the car on: the front-end platform file, which of its solids are drawn, how the car
//! stands in it and where the camera is. Spec: `docs/specs/menu-backdrop-looks.md`.

use game_install::GameDir;
use nfsmw_data::frontend_camera;

use super::camera_move::CameraMove;
use super::camera_pose;
use super::look::Look;

/// A front-end platform (`FRONTEND/PLATFORMS/*.BIN`, `docs/specs/menu-backdrop.md` section 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Platform 1: the crib, behind the main menu and the menus that open from it.
    Crib,
    /// Platform 2: the career safe house.
    SafeHouse,
}

/// How the original stands a car on a platform: the yaw of the car and the height its wheels rest at. [exe]
pub struct CarStand {
    /// Degrees the car is turned about the vertical axis, 0 having its front along +x.
    pub yaw: f32,
    /// Metres the car is lowered below the floor level its wheels would rest on, so the tyres sink in a little.
    pub sink: f32,
}

impl Look {
    /// The stage the look stands on.
    pub fn stage(&self) -> Stage {
        match self {
            Look::SafeHouse(_) => Stage::SafeHouse,
            Look::Main | Look::Car(_) | Look::Showcase(_) => Stage::Crib,
        }
    }
}

impl Stage {
    /// The platform file in the install.
    pub fn file(self) -> &'static str {
        match self {
            Stage::Crib => "FRONTEND/PLATFORMS/PlatformCrib.BIN",
            Stage::SafeHouse => "FRONTEND/PLATFORMS/CAREER_SAFEHOUSE.BIN",
        }
    }

    /// Whether the solid called `name` is drawn. The crib draws all of its five. The safe house file also holds the
    /// geometry the original renders its shadow maps from (`BACKDROP_CAST_SHADOW_MAP*`) and particle emitter markers
    /// (`STEAM EMITTER*`), which are not part of the picture.
    pub fn draws(self, name: &str) -> bool {
        match self {
            Stage::Crib => true,
            Stage::SafeHouse => !name.starts_with("BACKDROP_CAST_SHADOW_MAP") && !name.starts_with("STEAM EMITTER"),
        }
    }

    /// The camera move of the stage's menu, or `None` for the crib, whose camera is the car viewer's orbit camera. The
    /// safe house's runs from its entry pose onto the final one; an install whose poses cannot be read gets the final
    /// pose from the first frame.
    pub fn camera(self, dir: Option<&GameDir>) -> Option<CameraMove> {
        match self {
            Stage::Crib => None,
            Stage::SafeHouse => {
                Some(dir.and_then(safe_house_move).unwrap_or(CameraMove::holding(camera_pose::SAFE_HOUSE)))
            }
        }
    }

    /// How the car stands on the stage, or `None` for the crib, where the car viewer's own placement stays. [exe]
    pub fn car_stand(self) -> Option<CarStand> {
        match self {
            Stage::Crib => None,
            Stage::SafeHouse => Some(CarStand { yaw: 304.97, sink: 0.025 }),
        }
    }
}

/// The poses of the safe house's move (`FE_ATTRIB.bin`, class `frontend`): where the camera is when the menu opens and
/// where it ends up.
const SAFE_HOUSE_POSES: [&str; 2] = ["angle_entry_career_safehouse", "angle_final_career_safehouse"];

fn safe_house_move(dir: &GameDir) -> Option<CameraMove> {
    match frontend_camera::load(dir, &SAFE_HOUSE_POSES) {
        Ok(poses) => Some(CameraMove::between(&poses[0], &poses[1])),
        Err(e) => {
            log::warn!("the safe house camera holds its final pose: {e:#}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::look::CarLook;

    #[test]
    fn the_looks_pick_their_stage() {
        assert_eq!(Look::Main.stage(), Stage::Crib);
        assert_eq!(Look::Car(CarLook::default_car()).stage(), Stage::Crib);
        assert_eq!(Look::SafeHouse(CarLook::default_car()).stage(), Stage::SafeHouse);
    }

    #[test]
    fn the_safe_house_leaves_out_its_shadow_casters_and_emitters() {
        assert!(Stage::SafeHouse.draws("BACKDROP_CRIB"));
        assert!(Stage::SafeHouse.draws("BACKDROP_REFLECTION"));
        assert!(!Stage::SafeHouse.draws("BACKDROP_CAST_SHADOW_MAP2"));
        assert!(!Stage::SafeHouse.draws("STEAM EMITTER03"));
        assert!(Stage::Crib.draws("BACKDROP_BACKDROP_CAST_SHADOW_MAP2"));
    }

    #[test]
    fn only_the_safe_house_has_its_own_camera_and_car_stand() {
        assert!(Stage::Crib.camera(None).is_none() && Stage::Crib.car_stand().is_none());
        assert!(Stage::SafeHouse.camera(None).is_some() && Stage::SafeHouse.car_stand().is_some());
    }
}
