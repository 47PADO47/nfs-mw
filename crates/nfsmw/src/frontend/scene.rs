//! The scenes the front end puts in the window: an empty backdrop for the menus, and a wrapper that freezes the
//! driving scene while the pause menu is up.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use blackbox_render::{FrameParams, Instance, Renderer};
use glam::{Mat4, Vec3};

use crate::input::ActionState;
use crate::viewer::{Fullscreen, Scene};

/// What the menus sit on until the front end has a 3D backdrop of its own: a dark flat colour.
pub struct MenuScene;

impl Scene for MenuScene {
    fn title(&self) -> String {
        "nfsmw".to_owned()
    }

    fn init(&mut self, _renderer: &mut Renderer) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, _renderer: &mut Renderer, _input: &ActionState, _dt: f32) {}

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        Some(crate::hud::HudState { visible: false, ..Default::default() })
    }

    fn frame(&mut self, _aspect: f32) -> (FrameParams, &[Instance]) {
        let params = FrameParams {
            view_proj: Mat4::IDENTITY,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.02, 0.02, 0.03],
            fog_start: f32::MAX,
            fog_end: f32::MAX,
        };
        (params, &[])
    }
}

/// Whether a [`Pausable`] scene is frozen. The front end keeps one end, the scene the other.
#[derive(Clone, Default)]
pub struct PauseFlag(Arc<AtomicBool>);

impl PauseFlag {
    pub fn set(&self, paused: bool) {
        self.0.store(paused, Ordering::Relaxed);
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// A scene that stops while paused: it is not updated, makes no engine sound, takes no mouse and shows no HUD,
/// but is still drawn, so the pause menu has the frozen road behind it.
pub struct Pausable {
    inner: Box<dyn Scene>,
    paused: PauseFlag,
    effects_dirty: bool,
}

impl Pausable {
    pub fn new(inner: Box<dyn Scene>, paused: PauseFlag) -> Self {
        Self { inner, paused, effects_dirty: false }
    }
}

impl Scene for Pausable {
    fn title(&self) -> String {
        self.inner.title()
    }

    fn init(&mut self, renderer: &mut Renderer) -> Result<()> {
        self.inner.init(renderer)
    }

    fn update(&mut self, renderer: &mut Renderer, input: &ActionState, dt: f32) {
        if self.paused.get() {
            if self.effects_dirty {
                self.inner.refresh_effects(renderer);
                self.effects_dirty = false;
            }
            return;
        }
        self.inner.update(renderer, input, dt);
        self.effects_dirty = false;
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        self.inner.frame(aspect)
    }

    fn ready(&self) -> bool {
        self.inner.ready()
    }

    fn captures_mouse(&self) -> bool {
        !self.paused.get() && self.inner.captures_mouse()
    }

    fn status(&self) -> Option<String> {
        self.inner.status()
    }

    fn hud(&self) -> Option<String> {
        self.inner.hud()
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        if self.paused.get() {
            return Some(crate::hud::HudState { visible: false, ..Default::default() });
        }
        self.inner.hud_state()
    }

    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.paused.get() {
            return None;
        }
        self.inner.car_sound()
    }

    fn paused(&self) -> bool {
        self.paused.get()
    }

    fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
        self.inner.set_tire_effects(smoke, marks);
        self.effects_dirty = true;
    }

    fn set_smoke_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.inner.set_smoke_quality(quality);
        self.effects_dirty = true;
    }

    fn fullscreen(&mut self) -> Option<Fullscreen> {
        self.inner.fullscreen()
    }

    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        self.inner.take_clip()
    }

    fn finished(&self) -> bool {
        self.inner.finished()
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        self.inner.commands()
    }

    fn command(&mut self, renderer: &mut Renderer, name: &str, args: &[&str]) -> Option<Result<String, String>> {
        self.inner.command(renderer, name, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paused_game_is_still_a_game_for_the_radio() {
        let flag = PauseFlag::default();
        let mut scene = Pausable::new(Box::new(MenuScene), flag.clone());
        assert!(!scene.paused());
        flag.set(true);
        assert!(scene.paused());
        assert!(scene.car_sound().is_none(), "the car is silent while paused");
        assert!(!MenuScene.paused(), "the menus are not a game");
    }

    #[test]
    fn replacement_driving_scene_inherits_live_effect_settings_through_pause_wrapper() {
        use crate::app::Host;
        use crate::settings::{Partial, Settings, SmokeQuality};
        use std::sync::Mutex;

        #[derive(Default)]
        struct Seen {
            tires: [bool; 2],
            quality: Option<SmokeQuality>,
        }
        struct Observed(Arc<Mutex<Seen>>);
        impl Scene for Observed {
            fn title(&self) -> String {
                String::new()
            }
            fn init(&mut self, _: &mut Renderer) -> Result<()> {
                Ok(())
            }
            fn update(&mut self, _: &mut Renderer, _: &ActionState, _: f32) {}
            fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
                unreachable!()
            }
            fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
                self.0.lock().unwrap().tires = [smoke, marks];
            }
            fn set_smoke_quality(&mut self, quality: SmokeQuality) {
                self.0.lock().unwrap().quality = Some(quality);
            }
        }
        let settings = Settings::from(Partial::default());
        let mut host = Host::new(Box::new(MenuScene), &settings, None);
        host.set_tire_effects(false, true);
        host.set_smoke_quality(SmokeQuality::High);
        let seen = Arc::new(Mutex::new(Seen::default()));
        let scene = Pausable::new(Box::new(Observed(seen.clone())), PauseFlag::default());
        // Preferences must reach the incoming scene before renderer init and screenshot settling.
        assert!(host.replace_scene(Box::new(scene)).is_err(), "this test has no renderer");
        let seen = seen.lock().unwrap();
        assert_eq!(seen.tires, [false, true]);
        assert_eq!(seen.quality, Some(SmokeQuality::High));
    }
}
