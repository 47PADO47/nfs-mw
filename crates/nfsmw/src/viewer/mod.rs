//! What a viewer scene is, and the cameras scenes share. The window, loop and renderer belong to
//! [`crate::app`]; input arrives as actions ([`crate::input`]).

pub mod camera;

use anyhow::Result;
use blackbox_render::{FrameParams, Instance, Renderer};

use crate::input::ActionState;

/// How a [`Fullscreen`] picture meets the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
    /// Keep this width over height; black bands fill what is left of the window.
    Contain(f32),
    /// Stretch over the whole window, whatever its shape.
    Fill,
}

/// A picture a scene wants over the whole window (a movie frame).
pub struct Fullscreen {
    pub size: [u32; 2],
    /// The part of the picture to show, as `[u0, v0, u1, v1]` fractions (the rest is baked-in black bars).
    pub view: [f32; 4],
    pub fit: Fit,
    /// A new frame to upload (RGBA8, `size`); `None` keeps the one on screen.
    pub rgba: Option<Vec<u8>>,
}

pub trait Scene {
    fn title(&self) -> String;
    /// Upload the initial resources.
    fn init(&mut self, renderer: &mut Renderer) -> Result<()>;
    /// Advance by `dt` seconds: camera, streaming, uploads.
    fn update(&mut self, renderer: &mut Renderer, input: &ActionState, dt: f32);
    /// The frame to draw. Instances of the same mesh should be adjacent.
    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]);
    /// For screenshots: whether everything the first view needs has loaded.
    fn ready(&self) -> bool {
        true
    }
    /// Mouse look: hide and hold the cursor so mouse motion always turns the camera.
    /// Esc releases the cursor and a click captures it again.
    fn captures_mouse(&self) -> bool {
        false
    }
    /// Extra text for the window title (streaming progress, counts).
    fn status(&self) -> Option<String> {
        None
    }
    /// Lines for the scene's debug readout, drawn bottom left whatever the metrics level is: `level` says how
    /// much (the original HUD already shows speed, rpm and gear, so `Minimal` is one line that adds to it).
    /// Not called at `Off`.
    fn readout(&self, _level: crate::devtools::ShowReadout) -> Option<String> {
        None
    }
    /// Who changes gear (the transmission setting), told every frame before `update`; a scene without a car
    /// ignores it.
    fn set_transmission(&mut self, _transmission: crate::settings::Transmission) {}
    /// What the in-game HUD shows this frame; `None` hides it.
    fn hud_state(&self) -> Option<crate::hud::HudState> {
        None
    }
    /// The car being driven, for the engine sound; `None` is silence.
    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        None
    }
    /// The game is on but frozen (the pause menu is up): there is no car sound, yet the game is still being
    /// played, so the radio goes on.
    fn paused(&self) -> bool {
        false
    }
    /// Apply the independently layered tire-visual settings. Scenes without tires ignore them.
    fn set_tire_effects(&mut self, _smoke: bool, _skid_marks: bool) {}
    /// Apply optional collision sparks and high-speed wind trails.
    fn set_vehicle_effects(&mut self, _sparks: bool, _trails: bool) {}
    fn set_spark_style(&mut self, _style: crate::settings::SparkStyle) {}
    /// Show the flames at the car's tail pipes (the `exhaust_flames` setting). Scenes without a car ignore it.
    fn set_exhaust_flames(&mut self, _on: bool) {}
    /// The sound's sputter module started `pops` pops since the last call (the lift-off backfire follows them).
    fn note_sputters(&mut self, _pops: u32) {}
    /// Refresh changed visual-effect buffers without advancing a paused scene.
    fn refresh_effects(&mut self, _renderer: &mut Renderer) {}
    /// Apply the optional smoke presentation quality. Scenes without tires ignore it.
    fn set_smoke_quality(&mut self, _quality: crate::settings::SmokeQuality) {}
    /// A picture to show over the window this frame, if any.
    fn fullscreen(&mut self) -> Option<Fullscreen> {
        None
    }
    /// Sound the scene wants played once, handed over when asked (the audio of a movie).
    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        None
    }
    /// The scene is over: the app quits.
    fn finished(&self) -> bool {
        false
    }
    /// The scene's own console commands as `(name, usage)`, listed by `help`.
    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }
    /// Run a console command. `None` means the scene has no such command.
    fn command(&mut self, _renderer: &mut Renderer, _name: &str, _args: &[&str]) -> Option<Result<String, String>> {
        None
    }
}
