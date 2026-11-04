//! The platform's authored particle groups (the crib's steam and birds, the shop's smoke, the car lot's leaves): the stage
//! file's own emitter library, run by the world's ambient effect system. Spec: `docs/specs/menu-backdrop-looks.md`
//! ("Atmosphere").

use std::sync::Arc;

use blackbox_attrib::Database;
use blackbox_gfx::{EffectLayer, RenderBackend};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::world::emitter_placements::Placement;

use crate::scenes::world::ambient_fx::AmbientFx;

/// The live particle groups of one stage.
pub struct RoomFx {
    dir: GameDir,
    db: Arc<Database>,
    placements: Vec<Placement>,
    fx: AmbientFx,
    layer: EffectLayer,
}

impl RoomFx {
    /// The stage's groups, or `None` when it places none or the install's attributes are unusable.
    pub fn new(dir: GameDir, placements: Vec<Placement>) -> Option<Self> {
        if placements.is_empty() {
            return None;
        }
        let db = nfsmw_data::shared::attributes(&dir).ok()?;
        Some(Self { dir, db, placements, fx: AmbientFx::frontend(), layer: EffectLayer::default() })
    }

    /// Admits the groups in reach of the camera, advances them by `dt` and hands their particles to the renderer. The
    /// placements are in the unturned stage frame, like the car and the camera: the original's platform turn applies to
    /// the solids alone.
    pub fn update(&mut self, renderer: &mut dyn RenderBackend, camera: Vec3, forward: Vec3, dt: f32) {
        let placements = self.placements.iter().enumerate().map(|(i, placement)| ((0, i), placement));
        // The platform has no actors to cross a trigger: only its untriggered placements run.
        self.fx.maintain(renderer, &self.dir, &self.db, placements, &[(camera, forward)], &[]);
        self.fx.step(dt);
        self.layer.textured.clear();
        self.fx.textured(camera, forward, &mut self.layer.textured);
        renderer.set_effects(&self.layer);
    }

    /// Clears the particles from the renderer and frees the groups' textures.
    pub fn release(mut self, renderer: &mut dyn RenderBackend) {
        renderer.set_effects(&EffectLayer::default());
        self.fx.release(renderer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::room::RoomData;
    use crate::frontend::stage::Stage;

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn r6_installed_crib_places_steam_and_birds_that_reach_the_renderer() {
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
        let data = RoomData::read(&dir, Stage::Crib).unwrap();
        assert!(data.placements().len() >= 8, "the crib's steam and bird placements: {}", data.placements().len());
        let mut renderer =
            crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_texture_tracking();
        let mut fx = RoomFx::new(dir, data.placements().to_vec()).expect("the crib has particle groups");
        // The main menu's eye (`angle3`), looking at the car.
        let camera = Vec3::new(0.3, -5.4, 0.7);
        for _ in 0..240 {
            fx.update(&mut renderer, camera, Vec3::Y, 1.0 / 60.0);
        }
        assert!(!fx.layer.textured.is_empty(), "the crib's particles reach the renderer");
        fx.release(&mut renderer);
        assert_eq!(renderer.resources.lock().unwrap().textures, 0);
    }
}
