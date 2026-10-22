//! State the app's systems share. It is `NonSend`: the renderer and the scene stay on the main thread.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy_ecs::resource::Resource;
use blackbox_gfx::{CaptureId, RenderBackend};

use super::graphics::Applied;
use super::pacing::FrameLimiter;
use crate::settings::{CameraMode, CarShading, Settings, SmokeQuality, Transmission, WheelOptions};
use crate::viewer::Scene;

pub struct Host {
    pub scene: Box<dyn Scene>,
    /// The menu backdrop, kept initialised while free roam has the window, so the menus that come back reuse it
    /// (see [`Host::replace_scene_parking_old`] and [`Host::restore_parked_menu`]).
    pub parked_menu: Option<Box<dyn Scene>>,
    /// Counts the scenes put in the window, so a scene's sound can be stopped when it goes.
    pub scene_changes: u32,
    pub screenshot: Option<super::screenshot::Plan>,
    /// Created once the window exists; the app only knows the trait, the factory is in [`super::render`].
    pub renderer: Option<Box<dyn RenderBackend>>,
    /// The graphics request the renderer has and what it made of it (downgrades, notes).
    pub graphics: Applied,
    /// A screenshot the renderer is still working on: its id, its file and whether it is the last of its plan.
    pub pending_capture: Option<(CaptureId, PathBuf, bool)>,
    /// The window size the renderer was last resized to.
    pub size: (u32, u32),
    pub limiter: FrameLimiter,
    pub latency_pacing: bool,
    pub cpu_work_started: Instant,
    pub cpu_update_ms: f32,
    pub reflex_sleep_ms: f32,
    pub title_timer: Instant,
    pub frames: u32,
    /// `--bench-seconds`: measures frame times and exits.
    pub bench: Option<super::bench::Bench>,
    /// The front end uses Escape (pause, back), so Escape does not release the mouse or quit.
    pub cancel_handled: bool,
    /// The front end decides when a scene is over (a finished movie is not the end of the program).
    pub flow_driven: bool,
    /// A screenshot run waits while this is set (a scripted menu is still running).
    pub hold_capture: bool,
    /// The rear-view mirror drawn over the HUD (see [`super::mirror`]).
    pub mirror: super::mirror::MirrorOverlay,
    pub streaks: super::streaks::StreakOverlay,
    /// The two halves of a split-screen race (see [`super::split`]).
    pub split: super::split::SplitOverlay,
    /// The `rear_view_mirror` setting as of the last frame: when off, the rear view is not captured.
    pub rear_view_mirror: bool,
    tire_effects: [bool; 2],
    smoke_quality: SmokeQuality,
    rain: (bool, f32),
    world: crate::settings::WorldTuning,
    car_shading: CarShading,
    vehicle_effects: [bool; 2],
    spark_style: crate::settings::SparkStyle,
    speed_trail_style: crate::settings::SpeedTrailStyle,
    exhaust_flames: bool,
    /// The transmission setting as of the last frame, given to every scene that is put in the window.
    pub transmission: Transmission,
    /// The gameplay camera setting as of the last frame, given to every scene that is put in the window.
    pub camera_mode: CameraMode,
    /// The wheel switches as of the last frame, given to every scene that is put in the window.
    pub wheel: WheelOptions,
    /// The traffic settings as of the last frame, given to every scene that is put in the window.
    pub traffic: crate::settings::TrafficSettings,
}

impl Host {
    pub fn new(mut scene: Box<dyn Scene>, settings: &Settings, screenshot: Option<super::screenshot::Plan>) -> Self {
        scene.set_tire_effects(settings.tire_smoke, settings.skid_marks);
        scene.set_smoke_quality(settings.smoke_quality);
        scene.set_rain(settings.always_rain, settings.rain_amount.amplitude());
        scene.set_world_tuning(settings.into());
        scene.set_detail(settings.detail);
        scene.set_car_shading(settings.car_shading);
        scene.set_vehicle_effects(settings.collision_sparks, settings.speed_trails);
        scene.set_spark_style(settings.spark_style);
        scene.set_speed_trail_style(settings.speed_trail_style);
        scene.set_exhaust_flames(settings.exhaust_flames);
        Self {
            scene,
            parked_menu: None,
            scene_changes: 0,
            screenshot,
            renderer: None,
            graphics: Applied::default(),
            pending_capture: None,
            size: (0, 0),
            limiter: FrameLimiter::new(settings.max_fps),
            latency_pacing: false,
            cpu_work_started: Instant::now(),
            cpu_update_ms: 0.0,
            reflex_sleep_ms: 0.0,
            title_timer: Instant::now(),
            frames: 0,
            bench: None,
            cancel_handled: false,
            flow_driven: false,
            hold_capture: false,
            mirror: Default::default(),
            streaks: Default::default(),
            split: Default::default(),
            rear_view_mirror: settings.rear_view_mirror,
            tire_effects: [settings.tire_smoke, settings.skid_marks],
            smoke_quality: settings.smoke_quality,
            rain: (settings.always_rain, settings.rain_amount.amplitude()),
            world: settings.into(),
            car_shading: settings.car_shading,
            vehicle_effects: [settings.collision_sparks, settings.speed_trails],
            spark_style: settings.spark_style,
            speed_trail_style: settings.speed_trail_style,
            exhaust_flames: settings.exhaust_flames,
            transmission: settings.transmission,
            camera_mode: settings.camera_mode,
            wheel: settings.wheel_options(),
            traffic: crate::settings::TrafficSettings::from(settings),
        }
    }

    /// Measure frame times for `seconds` and exit (`--bench-seconds`).
    pub fn with_bench(mut self, seconds: Option<f32>) -> Self {
        self.bench = seconds.map(super::bench::Bench::new);
        self
    }

    /// Puts another scene in the window: it is initialised with the renderer, and in a screenshot run it is given
    /// the time to load. The scene it replaces is closed.
    pub fn replace_scene(&mut self, scene: Box<dyn Scene>) -> anyhow::Result<()> {
        let mut old = self.install(scene)?;
        self.close_scene(old.as_mut());
        Ok(())
    }

    /// Like [`Host::replace_scene`], but the scene it replaces is parked for [`Host::restore_parked_menu`] instead of
    /// closed. Only one scene is parked: a menu parked before is closed now.
    pub fn replace_scene_parking_old(&mut self, scene: Box<dyn Scene>) -> anyhow::Result<()> {
        let old = self.install(scene)?;
        let stale = self.parked_menu.replace(old);
        if let Some(mut stale) = stale {
            self.close_scene(stale.as_mut());
        }
        Ok(())
    }

    /// Puts the parked menu back in the window in place of the scene there, which is closed. The menu keeps the
    /// resources it was built with, so it is not initialised again; it is told it is back on screen (`Scene::reattached`).
    /// Returns `false`, with nothing changed, when no menu is parked.
    pub fn restore_parked_menu(&mut self) -> bool {
        let Some(mut menu) = self.parked_menu.take() else { return false };
        self.apply_scene_settings(menu.as_mut());
        self.apply_driving_settings(menu.as_mut());
        let Some(renderer) = self.renderer.as_mut() else {
            self.parked_menu = Some(menu);
            return false;
        };
        renderer.set_colour_treatment(None, None);
        renderer.set_focus_mask(None);
        renderer.set_reflection_scene(None);
        renderer.set_projected_reflection_scene(None);
        let mut old = std::mem::replace(&mut self.scene, menu);
        self.scene_changes += 1;
        old.close(renderer.as_mut());
        self.scene.reattached(renderer.as_mut());
        true
    }

    /// Initialises `scene` and puts it in the window. Returns the scene it replaced, which the caller closes or
    /// parks. On error the window keeps its scene.
    fn install(&mut self, mut scene: Box<dyn Scene>) -> anyhow::Result<Box<dyn Scene>> {
        self.apply_scene_settings(scene.as_mut());
        self.init_scene(scene.as_mut())?;
        self.apply_driving_settings(scene.as_mut());
        self.wait_for_screenshot(scene.as_mut());
        self.scene_changes += 1;
        Ok(std::mem::replace(&mut self.scene, scene))
    }

    /// Resets the renderer for a new scene and lets the scene upload its resources. A scene that fails to upload
    /// frees what it got.
    fn init_scene(&mut self, scene: &mut dyn Scene) -> anyhow::Result<()> {
        let renderer = self.renderer.as_mut().ok_or_else(|| anyhow::anyhow!("the renderer is not ready"))?;
        renderer.set_colour_treatment(None, None);
        renderer.set_focus_mask(None);
        // A local environment belongs to the old scene; menus and viewers must not inherit its capture.
        renderer.set_reflection_scene(None);
        renderer.set_projected_reflection_scene(None);
        if let Err(e) = scene.init(renderer.as_mut()) {
            scene.close(renderer.as_mut());
            return Err(e);
        }
        Ok(())
    }

    fn wait_for_screenshot(&mut self, scene: &mut dyn Scene) {
        if self.screenshot.is_none() {
            return;
        }
        let Some(renderer) = self.renderer.as_mut() else { return };
        super::screenshot::wait_ready(scene, renderer.as_mut());
    }

    /// Frees what a scene leaving the window uploaded.
    fn close_scene(&mut self, scene: &mut dyn Scene) {
        let Some(renderer) = self.renderer.as_mut() else { return };
        scene.close(renderer.as_mut());
    }

    /// The settings every scene is given before its init, as the host holds them now.
    fn apply_scene_settings(&self, scene: &mut dyn Scene) {
        scene.set_tire_effects(self.tire_effects[0], self.tire_effects[1]);
        scene.set_smoke_quality(self.smoke_quality);
        scene.set_rain(self.rain.0, self.rain.1);
        scene.set_world_tuning(self.world);
        scene.set_car_shading(self.car_shading);
        scene.set_vehicle_effects(self.vehicle_effects[0], self.vehicle_effects[1]);
        scene.set_spark_style(self.spark_style);
        scene.set_speed_trail_style(self.speed_trail_style);
        scene.set_exhaust_flames(self.exhaust_flames);
    }

    /// The driving settings every scene is given after its init, as the host holds them now.
    fn apply_driving_settings(&self, scene: &mut dyn Scene) {
        scene.set_transmission(self.transmission);
        scene.set_camera_mode(self.camera_mode);
        scene.set_wheel_options(self.wheel);
        scene.set_traffic(self.traffic);
    }

    /// Give the renderer the graphics settings (once per change).
    pub fn apply_graphics(&mut self, settings: &Settings) {
        let Some(renderer) = self.renderer.as_mut() else { return };
        self.graphics.apply(renderer.as_mut(), settings);
    }

    pub fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
        self.tire_effects = [smoke, marks];
        self.scene.set_tire_effects(smoke, marks);
    }

    /// Gives the scene the world settings (game speed, car scale, the motion blur ...) when they changed.
    pub fn set_world_tuning(&mut self, tuning: crate::settings::WorldTuning) {
        if self.world == tuning {
            return;
        }
        self.world = tuning;
        self.scene.set_world_tuning(tuning);
    }

    pub fn set_rain(&mut self, always: bool, amount: f32) {
        self.rain = (always, amount);
        self.scene.set_rain(always, amount);
    }

    pub fn set_smoke_quality(&mut self, quality: SmokeQuality) {
        self.smoke_quality = quality;
        self.scene.set_smoke_quality(quality);
    }

    pub fn set_car_shading(&mut self, shading: CarShading) {
        self.car_shading = shading;
        self.scene.set_car_shading(shading);
    }

    pub fn set_spark_style(&mut self, style: crate::settings::SparkStyle) {
        self.spark_style = style;
        self.scene.set_spark_style(style);
    }

    pub fn set_speed_trail_style(&mut self, style: crate::settings::SpeedTrailStyle) {
        self.speed_trail_style = style;
        self.scene.set_speed_trail_style(style);
    }

    pub fn set_exhaust_flames(&mut self, on: bool) {
        self.exhaust_flames = on;
        self.scene.set_exhaust_flames(on);
    }

    pub fn set_vehicle_effects(&mut self, sparks: bool, trails: bool) {
        self.vehicle_effects = [sparks, trails];
        self.scene.set_vehicle_effects(sparks, trails);
    }
}

/// Where a failing system leaves its error for [`super::run`] to return.
#[derive(Resource, Clone)]
pub struct ErrorSlot(pub Arc<Mutex<Option<anyhow::Error>>>);

impl ErrorSlot {
    pub fn set(&self, error: anyhow::Error) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
    }

    pub fn take(&self) -> Option<anyhow::Error> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
