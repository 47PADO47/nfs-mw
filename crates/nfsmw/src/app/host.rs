//! State the app's systems share. It is `NonSend`: the renderer and the scene stay on the main thread.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use bevy_ecs::resource::Resource;
use blackbox_render::Renderer;

use super::pacing::FrameLimiter;
use crate::settings::{CarShading, Settings, SmokeQuality, Transmission, WheelOptions};
use crate::viewer::Scene;

pub struct Host {
    pub scene: Box<dyn Scene>,
    /// Counts the scenes put in the window, so a scene's sound can be stopped when it goes.
    pub scene_changes: u32,
    pub screenshot: Option<super::screenshot::Plan>,
    /// Created once the window exists.
    pub renderer: Option<Renderer>,
    /// The window size the renderer was last resized to.
    pub size: (u32, u32),
    pub limiter: FrameLimiter,
    pub title_timer: Instant,
    pub frames: u32,
    /// The front end uses Escape (pause, back), so Escape does not release the mouse or quit.
    pub cancel_handled: bool,
    /// The front end decides when a scene is over (a finished movie is not the end of the program).
    pub flow_driven: bool,
    /// A screenshot run waits while this is set (a scripted menu is still running).
    pub hold_capture: bool,
    tire_effects: [bool; 2],
    smoke_quality: SmokeQuality,
    car_shading: CarShading,
    vehicle_effects: [bool; 2],
    spark_style: crate::settings::SparkStyle,
    exhaust_flames: bool,
    /// The transmission setting as of the last frame, given to every scene that is put in the window.
    pub transmission: Transmission,
    /// The wheel switches as of the last frame, given to every scene that is put in the window.
    pub wheel: WheelOptions,
}

impl Host {
    pub fn new(mut scene: Box<dyn Scene>, settings: &Settings, screenshot: Option<super::screenshot::Plan>) -> Self {
        scene.set_tire_effects(settings.tire_smoke, settings.skid_marks);
        scene.set_smoke_quality(settings.smoke_quality);
        scene.set_car_shading(settings.car_shading);
        scene.set_vehicle_effects(settings.collision_sparks, settings.speed_trails);
        scene.set_spark_style(settings.spark_style);
        scene.set_exhaust_flames(settings.exhaust_flames);
        Self {
            scene,
            scene_changes: 0,
            screenshot,
            renderer: None,
            size: (0, 0),
            limiter: FrameLimiter::new(settings.max_fps),
            title_timer: Instant::now(),
            frames: 0,
            cancel_handled: false,
            flow_driven: false,
            hold_capture: false,
            tire_effects: [settings.tire_smoke, settings.skid_marks],
            smoke_quality: settings.smoke_quality,
            car_shading: settings.car_shading,
            vehicle_effects: [settings.collision_sparks, settings.speed_trails],
            spark_style: settings.spark_style,
            exhaust_flames: settings.exhaust_flames,
            transmission: settings.transmission,
            wheel: settings.wheel_options(),
        }
    }

    /// Puts another scene in the window: it is initialised with the renderer, and in a screenshot run it is given
    /// the time to load.
    pub fn replace_scene(&mut self, mut scene: Box<dyn Scene>) -> anyhow::Result<()> {
        scene.set_tire_effects(self.tire_effects[0], self.tire_effects[1]);
        scene.set_smoke_quality(self.smoke_quality);
        scene.set_car_shading(self.car_shading);
        scene.set_vehicle_effects(self.vehicle_effects[0], self.vehicle_effects[1]);
        scene.set_spark_style(self.spark_style);
        scene.set_exhaust_flames(self.exhaust_flames);
        let renderer = self.renderer.as_mut().ok_or_else(|| anyhow::anyhow!("the renderer is not ready"))?;
        scene.init(renderer)?;
        scene.set_transmission(self.transmission);
        scene.set_wheel_options(self.wheel);
        if self.screenshot.is_some() {
            super::screenshot::wait_ready(scene.as_mut(), renderer);
        }
        self.scene = scene;
        self.scene_changes += 1;
        Ok(())
    }

    pub fn set_tire_effects(&mut self, smoke: bool, marks: bool) {
        self.tire_effects = [smoke, marks];
        self.scene.set_tire_effects(smoke, marks);
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
