//! The loading screen of free roam (docs/specs/loading-screen.md). The world is started behind the install's own
//! `WS_Loading.fng` package and is concealed (drawn and heard by nothing) until `Scene::ready` says the start area
//! has streamed in; then the front end reveals it and goes on to driving.

use super::scene::ConcealFlag;

/// Whether the start is over: the scene is ready and no race is still asked for or on its way to the grid (a career race is part
/// of the start, `docs/specs/loading-screen.md` section 6).
pub fn start_over(scene_ready: bool, race_staging: bool) -> bool {
    scene_ready && !race_staging
}

/// One frame of the loading stage. Returns whether loading is over; when it is, the world is revealed.
pub fn step(conceal: &ConcealFlag, scene_ready: bool) -> bool {
    if !scene_ready {
        return false;
    }
    conceal.set(false);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::flow::free_roam_options;

    #[test]
    fn the_world_stays_concealed_until_the_scene_is_ready() {
        let conceal = ConcealFlag::default();
        conceal.set(true);
        assert!(!step(&conceal, false));
        assert!(!step(&conceal, false), "still loading: the world stays hidden");
        assert!(conceal.get());
        assert!(step(&conceal, true), "ready: loading is over");
        assert!(!conceal.get(), "and the world is revealed");
    }

    #[test]
    fn a_race_still_staging_keeps_the_start_open() {
        assert!(!start_over(true, true), "the car is on its way to the grid: the screen stays");
        assert!(!start_over(false, false));
        assert!(start_over(true, false));
    }

    #[test]
    fn the_veil_passes_the_staging_of_the_world_on() {
        use crate::viewer::Scene;
        struct Staging;
        impl Scene for Staging {
            fn title(&self) -> String {
                String::new()
            }
            fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
                Ok(())
            }
            fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &crate::input::ActionState, _: f32) {}
            fn frame(&mut self, _: f32) -> (blackbox_gfx::FrameParams, &[blackbox_gfx::Instance]) {
                (crate::frontend::scene::flat_frame(), &[] as &[blackbox_gfx::Instance])
            }
            fn race_staging(&self) -> bool {
                true
            }
        }
        let scene = crate::frontend::scene::Pausable::new(Box::new(Staging), Default::default());
        assert!(scene.race_staging());
    }

    #[test]
    fn free_roam_waits_for_its_start_area_to_load() {
        assert!(
            free_roam_options(crate::scenes::world::DEFAULT_CAR).wait_for_load,
            "without it the world is shown before it has streamed"
        );
    }

    #[test]
    fn a_concealed_scene_shows_and_sounds_nothing_and_reports_its_own_readiness() {
        use crate::frontend::scene::{Pausable, PauseFlag, flat_frame};
        use crate::input::ActionState;
        use crate::viewer::Scene;
        use blackbox_gfx::{FrameParams, Instance};
        use std::sync::Arc;
        use std::sync::Mutex;

        struct Silent(Arc<Mutex<bool>>);
        impl Scene for Silent {
            fn title(&self) -> String {
                String::new()
            }
            fn init(&mut self, _: &mut dyn blackbox_gfx::RenderBackend) -> anyhow::Result<()> {
                Ok(())
            }
            fn update(&mut self, _: &mut dyn blackbox_gfx::RenderBackend, _: &ActionState, _: f32) {}
            fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
                (flat_frame(), &[] as &[Instance])
            }
            fn ready(&self) -> bool {
                *self.0.lock().unwrap()
            }
        }
        let ready = Arc::new(Mutex::new(false));
        let conceal = ConcealFlag::default();
        conceal.set(true);
        let mut scene =
            Pausable::new(Box::new(Silent(ready.clone())), PauseFlag::default()).with_conceal(conceal.clone());
        assert!(scene.hud_state().is_some_and(|h| !h.visible), "no HUD over the loading screen");
        assert!(scene.car_sound().is_none(), "the car is silent until the world is shown");
        assert!(scene.rear_view(2.0).is_none(), "the mirror is not drawn over the loading screen");
        assert!(!scene.captures_mouse());
        assert!(!scene.ready());
        *ready.lock().unwrap() = true;
        assert!(scene.ready(), "readiness is the scene's, whatever the veil");
        conceal.set(false);
        assert!(scene.hud_state().is_none(), "once revealed, the scene's own HUD answer comes back");
    }
}
