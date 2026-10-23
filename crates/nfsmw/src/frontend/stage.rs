//! The stage a backdrop look stands the car on: the front-end platform file, which of its solids are drawn, how the car
//! stands in it and where the camera is. Spec: `docs/specs/menu-backdrop-looks.md`.

use super::look::Look;

/// A front-end platform (`FRONTEND/PLATFORMS/*.BIN`, `docs/specs/menu-backdrop.md` section 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Platform 1: the crib, behind the main menu and the menus that open from it.
    Crib,
    /// Platform 2: the career safe house.
    SafeHouse,
    /// Platform 5: the car lot, its own room, which the lot zones and the first car's purchase open.
    CarLot,
}

/// How the original stands a car on a platform: the yaw of the car and the height its wheels rest at. [exe]
pub struct CarStand {
    /// Degrees the car is turned about the vertical axis, 0 having its front along +x.
    pub yaw: f32,
    /// Metres the car is lowered below the floor level its wheels would rest on, so the tyres sink in a little.
    pub sink: f32,
}

/// How the car stands on the crib and the safe house, which share it. [exe]
pub const CAR_STAND: CarStand = CarStand { yaw: 304.97, sink: 0.025 };

/// How the car stands in the car lot: the stand routine turns it 340 degrees there (`menu-backdrop-looks.md` section 4.1).
/// [exe]
pub const CAR_LOT_STAND: CarStand = CarStand { yaw: 340.0, sink: 0.025 };

impl Look {
    /// The stage the look stands on.
    pub fn stage(&self) -> Stage {
        match self {
            Look::SafeHouse(_) => Stage::SafeHouse,
            Look::CarLot(_) | Look::LotShowcase(_) => Stage::CarLot,
            Look::Main | Look::Car(_) | Look::Showcase(_) => Stage::Crib,
        }
    }

    /// The name of the look's camera intro, which runs from `angle_entry_<name>` onto `angle_final_<name>`, or `None` for
    /// the Showcase, whose camera the player turns. [exe] The original names the crib's by the front end's context: the
    /// career menu's is `career_manager`, the others' `main_fe`.
    pub fn camera_intro(&self) -> Option<&'static str> {
        match self {
            Look::Main => Some("main_fe"),
            Look::Car(_) => Some("career_manager"),
            Look::SafeHouse(_) => Some("career_safehouse"),
            Look::CarLot(_) => Some("car_lot"),
            Look::Showcase(_) | Look::LotShowcase(_) => None,
        }
    }
}

impl Stage {
    /// The platform file in the install.
    pub fn file(self) -> &'static str {
        match self {
            Stage::Crib => "FRONTEND/PLATFORMS/PlatformCrib.BIN",
            Stage::SafeHouse => "FRONTEND/PLATFORMS/CAREER_SAFEHOUSE.BIN",
            Stage::CarLot => "FRONTEND/PLATFORMS/CAR_LOT.BIN",
        }
    }

    /// How a car stands on this stage.
    pub fn car_stand(self) -> &'static CarStand {
        match self {
            Stage::CarLot => &CAR_LOT_STAND,
            Stage::Crib | Stage::SafeHouse => &CAR_STAND,
        }
    }

    /// Degrees the room is turned about the vertical axis from where its file puts it, so the camera poses of the original
    /// (which turn about the car) see the room as its pictures show it. [unverified] Fitted against pictures of the original's
    /// main menu and career menu, where the crib's houses stand behind the car; the executable gives no such turn. The
    /// safe house needs none.
    pub fn room_yaw(self) -> f32 {
        match self {
            Stage::Crib => 180.0,
            Stage::SafeHouse | Stage::CarLot => 0.0,
        }
    }

    /// Whether the solid called `name` is drawn. The crib draws all of its five. The safe house file also holds the high
    /// resolution geometry the original renders its shadow map from (`BACKDROP_CAST_SHADOW_MAP_NEXTGEN`, above the room) and
    /// particle emitter markers (`STEAM EMITTER*`), which are not part of the picture. The three other `BACKDROP_CAST_SHADOW_MAP*`
    /// solids are inside the room and hold its props and wall parts (the couch, the tank, the posters, the pipes), which the
    /// mirrored garage repeats, so they are drawn.
    pub fn draws(self, name: &str) -> bool {
        match self {
            Stage::Crib | Stage::CarLot => true,
            Stage::SafeHouse => {
                !name.starts_with("BACKDROP_CAST_SHADOW_MAP_NEXTGEN") && !name.starts_with("STEAM EMITTER")
            }
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
        assert_eq!(Look::CarLot(CarLook::default_car()).stage(), Stage::CarLot);
    }

    #[test]
    fn the_car_lot_is_its_own_room_and_turns_the_car_340_degrees() {
        assert_eq!(Stage::CarLot.file(), "FRONTEND/PLATFORMS/CAR_LOT.BIN");
        assert_eq!(Look::CarLot(CarLook::default_car()).camera_intro(), Some("car_lot"));
        assert_eq!(Stage::CarLot.car_stand().yaw, 340.0);
        assert_eq!(Stage::SafeHouse.car_stand().yaw, 304.97);
    }

    #[test]
    fn the_safe_house_leaves_out_its_high_resolution_shadow_caster_and_emitters() {
        assert!(Stage::SafeHouse.draws("BACKDROP_CRIB"));
        assert!(Stage::SafeHouse.draws("BACKDROP_REFLECTION"));
        assert!(Stage::SafeHouse.draws("BACKDROP_CAST_SHADOW_MAP2"));
        assert!(!Stage::SafeHouse.draws("BACKDROP_CAST_SHADOW_MAP_NEXTGEN"));
        assert!(!Stage::SafeHouse.draws("STEAM EMITTER03"));
        assert!(Stage::Crib.draws("BACKDROP_BACKDROP_CAST_SHADOW_MAP2"));
    }

    #[test]
    fn every_look_but_the_showcase_has_a_camera_intro() {
        let car = CarLook::default_car;
        assert_eq!(Look::Main.camera_intro(), Some("main_fe"));
        assert_eq!(Look::Car(car()).camera_intro(), Some("career_manager"));
        assert_eq!(Look::SafeHouse(car()).camera_intro(), Some("career_safehouse"));
        assert_eq!(Look::Showcase(car()).camera_intro(), None);
    }
}
