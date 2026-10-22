//! Putting a scene in the window: swapping the scene, and showing the menu backdrop behind the menus.

use super::*;

impl Frontend {
    /// Puts `scene` in the window. With `park`, a menu backdrop being replaced is kept for the menus that come back
    /// (`Host::replace_scene_parking_old`); any other scene is released.
    pub(super) fn replace_scene(&mut self, host: &mut Host, scene: Box<dyn crate::viewer::Scene>, park: bool) -> bool {
        let parks_menu = park && self.menu_in_window;
        let result = match parks_menu {
            true => host.replace_scene_parking_old(scene),
            false => host.replace_scene(scene),
        };
        match result {
            Ok(()) => {
                self.menu_in_window = false;
                true
            }
            Err(e) => {
                log::error!("cannot change the scene: {e:#}");
                false
            }
        }
    }

    /// Puts the menu backdrop in the window. A backdrop already there is kept, and so is one parked while free roam
    /// had the window: building one reads the car from the install and uploads it (`MenuScene::init`, ~150-190 ms),
    /// which the menus that follow would only repeat.
    pub(super) fn show_backdrop(&mut self, host: &mut Host) {
        if self.menu_in_window {
            return;
        }
        if host.restore_parked_menu() {
            self.menu_in_window = true;
            return;
        }
        // A backdrop built from nothing plays its camera intro, whatever the camera did last.
        self.camera.reset();
        let scene = MenuScene::with_look(Some(self.dir.clone()), self.look.clone()).with_rig(self.camera.clone());
        if !self.replace_scene(host, Box::new(scene), false) {
            return;
        }
        self.menu_in_window = true;
    }
}
