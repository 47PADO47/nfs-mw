//! The scenes the front end puts in the window: an empty backdrop for the menus, and a wrapper that freezes the
//! driving scene while the pause menu is up.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, Projection, RenderBackend};
use glam::{Mat4, Vec3};

use crate::input::ActionState;
use crate::viewer::{Fullscreen, Scene};

/// What the menus sit on until the front end has a 3D backdrop of its own: a dark flat colour.
pub struct MenuScene;

impl Scene for MenuScene {
    fn title(&self) -> String {
        "nfsmw".to_owned()
    }

    fn init(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, _renderer: &mut dyn RenderBackend, _input: &ActionState, _dt: f32) {}

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        Some(crate::hud::HudState { visible: false, ..Default::default() })
    }

    fn frame(&mut self, _aspect: f32) -> (FrameParams, &[Instance]) {
        let params = FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.02, 0.02, 0.03],
            fog: None,
            // Nothing 3D is drawn, so there is no camera to follow from frame to frame.
            camera_cut: true,
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

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.inner.init(renderer)
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
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

    fn readout(&self, level: crate::devtools::ShowReadout) -> Option<String> {
        self.inner.readout(level)
    }

    fn set_transmission(&mut self, transmission: crate::settings::Transmission) {
        self.inner.set_transmission(transmission);
    }

    fn set_wheel_options(&mut self, wheel: crate::settings::WheelOptions) {
        self.inner.set_wheel_options(wheel);
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

    fn set_car_shading(&mut self, shading: crate::settings::CarShading) {
        self.inner.set_car_shading(shading);
    }

    fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
        self.inner.set_vehicle_effects(sparks, trails);
        self.effects_dirty = true;
    }

    fn set_spark_style(&mut self, style: crate::settings::SparkStyle) {
        self.inner.set_spark_style(style);
        self.effects_dirty = true;
    }

    fn set_exhaust_flames(&mut self, on: bool) {
        self.inner.set_exhaust_flames(on);
        self.effects_dirty = true;
    }

    fn note_sputters(&mut self, pops: u32) {
        self.inner.note_sputters(pops);
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

    fn command(
        &mut self,
        renderer: &mut dyn RenderBackend,
        name: &str,
        args: &[&str],
    ) -> Option<Result<String, String>> {
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
        use crate::settings::{CarShading, Partial, Settings, SmokeQuality, SparkStyle};
        use std::sync::Mutex;

        #[derive(Default)]
        struct Seen {
            tires: [bool; 2],
            quality: Option<SmokeQuality>,
            vehicle: [bool; 2],
            style: Option<SparkStyle>,
            shading: Option<CarShading>,
        }
        struct Observed(Arc<Mutex<Seen>>);
        impl Scene for Observed {
            fn title(&self) -> String {
                String::new()
            }
            fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
                Ok(())
            }
            fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {}
            fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
                unreachable!()
            }
            fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
                self.0.lock().unwrap().tires = [smoke, marks];
            }
            fn set_smoke_quality(&mut self, quality: SmokeQuality) {
                self.0.lock().unwrap().quality = Some(quality);
            }
            fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
                self.0.lock().unwrap().vehicle = [sparks, trails];
            }
            fn set_spark_style(&mut self, style: SparkStyle) {
                self.0.lock().unwrap().style = Some(style);
            }
            fn set_car_shading(&mut self, shading: CarShading) {
                self.0.lock().unwrap().shading = Some(shading);
            }
        }
        let settings = Settings::from(Partial::default());
        let mut host = Host::new(Box::new(MenuScene), &settings, None);
        host.set_tire_effects(false, true);
        host.set_smoke_quality(SmokeQuality::High);
        host.set_vehicle_effects(true, false);
        host.set_spark_style(SparkStyle::RestoredExperimental);
        host.set_car_shading(CarShading::Simple);
        let seen = Arc::new(Mutex::new(Seen::default()));
        let scene = Pausable::new(Box::new(Observed(seen.clone())), PauseFlag::default());
        // Preferences must reach the incoming scene before renderer init and screenshot settling.
        assert!(host.replace_scene(Box::new(scene)).is_err(), "this test has no renderer");
        let seen = seen.lock().unwrap();
        assert_eq!(seen.tires, [false, true]);
        assert_eq!(seen.quality, Some(SmokeQuality::High));
        assert_eq!(seen.vehicle, [true, false]);
        assert_eq!(seen.style, Some(SparkStyle::RestoredExperimental));
        assert_eq!(seen.shading, Some(CarShading::Simple));
    }

    #[test]
    fn paused_vehicle_effect_changes_forward_and_mark_buffers_for_refresh() {
        use std::sync::Mutex;

        struct Observed(Arc<Mutex<Vec<[bool; 2]>>>);
        impl Scene for Observed {
            fn title(&self) -> String {
                String::new()
            }
            fn init(&mut self, _: &mut dyn RenderBackend) -> Result<()> {
                Ok(())
            }
            fn update(&mut self, _: &mut dyn RenderBackend, _: &ActionState, _: f32) {
                panic!("a paused scene must not advance");
            }
            fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
                unreachable!()
            }
            fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
                self.0.lock().unwrap().push([sparks, trails]);
            }
        }
        let flag = PauseFlag::default();
        flag.set(true);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut scene = Pausable::new(Box::new(Observed(seen.clone())), flag);
        assert!(!scene.effects_dirty);
        scene.set_vehicle_effects(true, false);
        assert!(scene.effects_dirty, "the next paused frame must refresh the effect buffers");
        assert!(scene.paused());
        assert!(!scene.hud_state().unwrap().visible);
        scene.set_vehicle_effects(false, false);
        assert_eq!(*seen.lock().unwrap(), vec![[true, false], [false, false]]);
        assert!(scene.effects_dirty, "disabling also needs a refresh of frozen buffers");
        scene.effects_dirty = false;
        scene.set_spark_style(crate::settings::SparkStyle::RestoredExperimental);
        assert!(scene.effects_dirty, "a style change must refresh frozen sprite and streak buffers");
    }
}
