//! What a viewer scene is, and the cameras scenes share. The window, loop and renderer belong to
//! [`crate::app`]; input arrives as actions ([`crate::input`]).

pub mod camera;

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

use crate::hud::MapBlip;
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

/// Session-local world clock controls. The existing sky is not a full night renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Environment {
    pub hour: f32,
    pub seconds_per_hour: f32,
}

/// The player's car at one moment, in render space (x forward, y left, z up; metres and seconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCar {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    /// Heading, radians from +x towards +y.
    pub heading: f32,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    pub rpm: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// Nitrous left, 0 to 1; 0 without the part.
    pub nos: f32,
    /// Damage, 0 to 1 of the zones' levels.
    pub damage: f32,
}

/// The camera the player looks through, and the gameplay view it is in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraReadout {
    pub mode: crate::settings::CameraMode,
    pub position: [f32; 3],
    /// Unit length.
    pub forward: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
}

/// The track and the district the scene is in.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldInfo {
    pub track: String,
    /// The district at the player, `None` where the install names none.
    pub section: Option<String>,
}

pub trait Scene {
    fn environment(&self) -> Option<Environment> {
        None
    }
    fn set_environment(&mut self, _value: Environment) {}
    /// Whether a mod has held the hour (`bb.time` freeze). Scenes without a clock report false.
    fn time_held(&self) -> bool {
        false
    }
    fn set_time_held(&mut self, _held: bool) {}
    /// The minimap blips the mods placed (`bb.map`), merged into the scene's own. Scenes without a minimap ignore them.
    fn set_mod_blips(&mut self, _blips: &[MapBlip]) {}
    /// The player's car, as the mods read it (`bb.vehicles`). `None` where no car is driven. Takes `&mut` because the
    /// car's physics is reached through the drive.
    fn player_car(&mut self) -> Option<PlayerCar> {
        None
    }
    /// The gameplay camera, as the mods read it (`bb.camera`). `None` where the scene has no camera of its own.
    fn camera_readout(&self) -> Option<CameraReadout> {
        None
    }
    /// The track and the district of the scene (`bb.world`). `None` where no world is loaded.
    fn world_info(&self) -> Option<WorldInfo> {
        None
    }
    /// Move the player's car to the road nearest `at`, facing `heading`. Returns whether there was a car to move.
    fn move_player(&mut self, _at: [f32; 2], _heading: f32) -> bool {
        false
    }
    /// Repair the player's car. Returns whether there was a car to repair.
    fn repair_player(&mut self) -> bool {
        false
    }
    /// Keep `cars` computer-driven cars on the road, `cop_share` percent of them patrol cops.
    fn set_traffic_density(&mut self, _cars: u32, _cop_share: u8) {}

    /// Installed car choices for the runtime garage; scenes without driving return None.
    fn garage(&mut self) -> Option<&mut crate::garage::Model> {
        None
    }
    fn apply_garage(&mut self, _renderer: &mut dyn RenderBackend, _selection: crate::garage::Selection) -> Result<()> {
        anyhow::bail!("this scene has no garage")
    }
    /// Freeze driving while the garage owns the controls, allowing streaming and placement to continue.
    fn set_garage_open(&mut self, _open: bool) {}
    fn title(&self) -> String;
    /// Upload the initial resources.
    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()>;
    /// Advance by `dt` seconds: camera, streaming, uploads.
    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32);
    /// The frame to draw. Instances of the same mesh should be adjacent.
    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]);
    /// The rear-view mirror's frame, when the scene has a car to look back from. The mirror is drawn
    /// offscreen and shown in the HUD; the car itself is left out of it.
    /// The high-speed streaks' strength, 0 to 1 (`docs/specs/speed-effects.md`).
    fn speed_streaks(&self) -> f32 {
        0.0
    }
    fn rear_view(&mut self, _aspect: f32) -> Option<(FrameParams, &[Instance])> {
        None
    }
    /// For screenshots: whether everything the first view needs has loaded.
    fn ready(&self) -> bool {
        true
    }
    /// Start timing once resources are ready, even while a scripted route is still moving.
    fn benchmark_ready(&self) -> bool {
        self.ready()
    }
    /// Mouse look: hide and hold the cursor so mouse motion always turns the camera.
    /// Esc releases the cursor and a click captures it again.
    fn captures_mouse(&self) -> bool {
        false
    }
    /// The scene's own egui panels, drawn every UI pass over the window (after the scene's update, before the
    /// frame is drawn). Returns the width in points of the strip on the left that they take, so the overlays
    /// can sit beside them; 0 when there is none. Scenes without panels ignore it.
    fn ui(&mut self, _ctx: &egui::Context) -> f32 {
        0.0
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
    /// Which gameplay camera to use (the camera setting), told every frame; scenes without a chase camera ignore it.
    fn set_camera_mode(&mut self, _mode: crate::settings::CameraMode) {}
    /// Next camera accepted by this scene's ordinary gameplay cycle. Direct selections remain available.
    fn next_camera_mode(&self, current: crate::settings::CameraMode) -> crate::settings::CameraMode {
        current.next()
    }
    /// Whether the camera hotkey changes the gameplay camera now (only while driving).
    fn takes_camera_setting(&self) -> bool {
        false
    }
    /// A steering wheel's optional controls (the clutch pedal, the H-shifter), told every frame before `update`; a
    /// scene without a car ignores it.
    fn set_wheel_options(&mut self, _wheel: crate::settings::WheelOptions) {}
    /// How many computer-driven cars to keep around the player, what share of them are cops and whether they
    /// obey traffic lights, told every frame before `update`; a scene without roads ignores it.
    fn set_traffic(&mut self, _traffic: crate::settings::TrafficSettings) {}
    /// What the in-game HUD shows this frame; `None` hides it.
    fn hud_state(&self) -> Option<crate::hud::HudState> {
        None
    }
    /// The car being driven, for the engine sound; `None` is silence.
    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        None
    }
    /// The chase the cops run this frame, for the pursuit music and the speech; `None` without one.
    fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        None
    }
    /// The bust sequence's infractions ticket, once, when the front end should show it (`(infraction, fine)` lines);
    /// `None` otherwise.
    fn take_bust_ticket(&mut self) -> Option<Vec<(blackbox_pursuit::Infraction, u32)>> {
        None
    }
    /// The player pays the bust's fines on the ticket (`marker`: with a marker). `true` when the bust took the payment.
    fn pay_bust(&mut self, _marker: bool) -> bool {
        false
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
    /// Which high-speed effect the speed trails setting shows. Scenes without a car ignore it.
    fn set_speed_trail_style(&mut self, _style: crate::settings::SpeedTrailStyle) {}
    /// Show the flames at the car's tail pipes (the `exhaust_flames` setting). Scenes without a car ignore it.
    fn set_exhaust_flames(&mut self, _on: bool) {}
    /// The sound's sputter module started `pops` pops since the last call (the lift-off backfire follows them).
    fn note_sputters(&mut self, _pops: u32) {}
    /// Refresh changed visual-effect buffers without advancing a paused scene.
    fn refresh_effects(&mut self, _renderer: &mut dyn RenderBackend) {}
    /// Apply the optional smoke presentation quality. Scenes without tires ignore it.
    fn set_smoke_quality(&mut self, _quality: crate::settings::SmokeQuality) {}
    /// Apply the car shading setting: the scene re-uploads its cars when it changes. Scenes without cars ignore it.
    fn set_car_shading(&mut self, _shading: crate::settings::CarShading) {}
    /// Rain on every track, and its amount (0 to 1). Scenes without a road ignore it.
    fn set_rain(&mut self, _always: bool, _amount: f32) {}
    /// The world's speeds: the physics clock and the animation clock run at these multiples of the original rates. Scenes without a world ignore it.
    fn set_world_tuning(&mut self, _tuning: crate::settings::WorldTuning) {}
    /// A picture to show over the window this frame, if any.
    fn fullscreen(&mut self) -> Option<Fullscreen> {
        None
    }
    /// Sound the scene wants played once, handed over when asked (the audio of a movie).
    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        None
    }
    /// The one-shot sounds the traffic cars asked for since the last call (horns, near-miss whooshes).
    fn take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        Vec::new()
    }
    /// The police helicopter's rotor for this frame (`dt` seconds since the last one), or `None` without one.
    fn rotor_frame(&mut self, _dt: f32) -> Option<crate::audio::HeliFrame> {
        None
    }
    /// How many explosions (petrol-station blasts) started since the last call; each one plays its sound.
    fn take_explosions(&mut self) -> u32 {
        0
    }
    /// The sound viewer's transport, when the scene has one: the audio plugin plays what it asks for.
    fn sound_preview(&self) -> Option<crate::audio::preview::Preview> {
        None
    }
    /// Ordered global sound events since the last update.
    fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        Vec::new()
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
    fn command(
        &mut self,
        _renderer: &mut dyn RenderBackend,
        _name: &str,
        _args: &[&str],
    ) -> Option<Result<String, String>> {
        None
    }
}
