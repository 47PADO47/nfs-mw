//! Settings, layered: command line > environment > per-user config file > defaults.
//!
//! Each source produces a [`Partial`]; [`Settings::load`] merges them. The in-game settings menu
//! (milestone 6) and the developer console write the config file layer.

#[macro_use]
mod names;

pub mod availability;
mod camera_mode;
mod car_shading;
#[cfg(test)]
mod car_shading_tests;
mod controls;
#[cfg(test)]
mod controls_tests;
mod effective;
mod env;
#[cfg(test)]
mod exhaust_flames_tests;
mod extra_options;
pub(crate) mod file;
mod graphics;
mod graphics_preset;
#[cfg(test)]
mod graphics_preset_tests;
#[cfg(test)]
mod graphics_tests;
mod hud_layout;
#[cfg(test)]
mod hud_layout_tests;
mod minimap;
mod partial;
mod post;
#[cfg(test)]
mod post_tests;
mod radio_hud;
mod ray_tracing;
#[cfg(test)]
mod ray_tracing_tests;
#[cfg(test)]
mod rear_view_mirror_tests;
mod renderer;
#[cfg(test)]
mod renderer_tests;
mod smoke_quality;
#[cfg(test)]
mod smoke_quality_tests;
mod spark_style;
#[cfg(test)]
pub(crate) mod test_caps;
#[cfg(test)]
mod tire_tests;
mod transmission;
#[cfg(test)]
mod treatment_tests;
mod upscale;
mod upscale_quality;
#[cfg(test)]
mod upscale_quality_tests;
#[cfg(test)]
mod upscale_tests;
#[cfg(test)]
mod vehicle_effects_tests;
mod wheel;
mod window;
pub(crate) mod write;

use blackbox_gfx::GraphicsApi;

pub use blackbox_gfx::AoQuality;
pub use blackbox_gfx::ReflectionQuality;
pub use camera_mode::CameraMode;
pub use car_shading::CarShading;
pub use controls::{Controls, Deadzone, DeadzoneMode, Sensitivity};
pub use graphics_preset::GraphicsPreset;
pub use hud_layout::HudLayout;
pub use minimap::MinimapMode;
pub use partial::{Partial, Percent, parse_bool};
pub use post::{PostAa, PostBloom, PostTonemap, VisualTreatment, post_effects};
pub use radio_hud::RadioHudStyle;
pub use ray_tracing::RayTracingLevel;
pub use renderer::RendererKind;
pub use smoke_quality::SmokeQuality;
pub use spark_style::SparkStyle;
pub use transmission::Transmission;
pub use upscale::{RenderScale, UpscaleMode};
pub use upscale_quality::UpscaleQuality;
pub use wheel::WheelOptions;
pub use window::{Monitor, Resolution, WindowMode};
pub use write::write as write_file;

use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// Computer-driven cars kept on the road by default (the original keeps fewer than 10 vehicles active).
pub const DEFAULT_TRAFFIC: u32 = 10;
/// The default share of those cars that are patrol cops: 5 percent, one in twenty.
pub const DEFAULT_COP_SHARE: Percent = Percent(5);
/// Whether traffic stops at red lights by default. The original game has no traffic lights: this is a rewrite
/// extension, on because it was asked for.
pub const DEFAULT_TRAFFIC_LIGHTS: bool = true;
/// Rain on every track, as the Extra Options `[Weather] AlwaysRain` switch (off by default).
pub const DEFAULT_ALWAYS_RAIN: bool = false;
/// The rain amount: 100 percent is the original amount.
pub const DEFAULT_RAIN_AMOUNT: Percent = Percent(100);
/// The game speed multiplier: 1 is the original speed.
pub const DEFAULT_GAME_SPEED: f32 = 1.0;
/// The world's animation speed: 1 is the original rate (Extra Options `WorldAnimationSpeed`).
pub const DEFAULT_ANIMATION_SPEED: f32 = 1.0;
/// The car model scale: 1 is the original size (Extra Options `CarScale`, visual only).
pub const DEFAULT_CAR_SCALE: f32 = 1.0;
/// The rev limiter is on by default, as in the original.
pub const DEFAULT_NO_REV_LIMITER: bool = false;
/// The HUD stays up while looking back (Extra Options `ShowHUDOnLookBack`); off hides it, as the original does.
pub const DEFAULT_SHOW_HUD_ON_LOOK_BACK: bool = false;

/// The settings the world's traffic follows, told to the scene together so that it can see any of them change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrafficSettings {
    /// Cars kept on the road (0: none).
    pub cars: u32,
    /// Share of them that are patrol cops, in percent.
    pub cop_share: Percent,
    /// Whether they obey traffic lights.
    pub lights: bool,
}

impl From<&Settings> for TrafficSettings {
    fn from(s: &Settings) -> Self {
        Self { cars: s.traffic, cop_share: s.cop_share, lights: s.traffic_lights }
    }
}

/// The resolved settings.
#[derive(bevy_ecs::resource::Resource, Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub controls: Controls,
    pub backend: GraphicsApi,
    /// Who draws: the native `blackbox` renderer (default) or `bevy`; applies after a restart.
    pub renderer: RendererKind,
    pub vsync: bool,
    pub max_fps: MaxFps,
    pub show_metrics: ShowMetrics,
    pub window_mode: WindowMode,
    pub monitor: Monitor,
    pub resolution: Resolution,
    /// How much of the scene's debug readout is drawn next to the original HUD.
    pub show_readout: ShowReadout,
    pub master_volume: Percent,
    pub music_volume: Percent,
    pub sfx_volume: Percent,
    pub engine_volume: Percent,
    /// The police dispatch's radio lines.
    pub speech_volume: Percent,
    /// Draw the in-game HUD while driving (the free camera never shows it).
    pub hud: bool,
    /// Draw smoke from the driven car's loaded tire contacts.
    pub tire_smoke: bool,
    /// Play the radio while driving (the `radio` console command still works when it is off).
    pub radio: bool,
    /// Start at the main menu instead of the boot movies and the title screen (the `--skip-boot` flag).
    pub skip_intro: bool,
    /// The preset the settings it covers still match, else `Custom` (see [`GraphicsPreset`]).
    pub graphics_preset: GraphicsPreset,
    /// How cars are shaded: glossy (default) or the single-light shading of everything else.
    pub car_shading: CarShading,
    /// How many computer-driven cars the world keeps on the road around you (0 turns traffic off).
    pub traffic: u32,
    /// Of those cars, the share that are patrol cops, in percent.
    pub cop_share: Percent,
    /// Whether the traffic obeys traffic lights at junctions (a rewrite extension: the original has none).
    pub traffic_lights: bool,
    /// Rain on every track (Extra Options `AlwaysRain`).
    pub always_rain: bool,
    /// The rain amount, 0 to 100 percent of the original.
    pub rain_amount: Percent,
    /// The physics clock multiplier, 0.1 to 4 (Extra Options `GameSpeed`).
    pub game_speed: f32,
    /// The world animation rate, 0.1 to 4 (Extra Options `WorldAnimationSpeed`).
    pub animation_speed: f32,
    /// The player car's model scale, 0.25 to 4 (Extra Options `CarScale`).
    pub car_scale: f32,
    /// The HUD stays up while looking back (Extra Options `ShowHUDOnLookBack`).
    pub show_hud_on_look_back: bool,
    /// Lift the engine's rev limiter (Extra Options `NoRevLimiter`).
    pub no_rev_limiter: bool,
    /// Optional smoke presentation quality; standard retains the default cost and look.
    pub smoke_quality: SmokeQuality,
    /// Draw bounded, ground-following tire marks.
    pub skid_marks: bool,
    /// Collision particles, using the selected stock or experimental style.
    pub collision_sparks: bool,
    pub spark_style: SparkStyle,
    /// Experimental wind trails at high speed.
    pub speed_trails: bool,
    /// Flames at the tail pipes: gear-change blow-off and lift-off backfire. Nothing is loaded when off.
    pub exhaust_flames: bool,
    /// The rear-view mirror in the HUD (the rear view captured from the car, with the radar under it).
    pub rear_view_mirror: bool,
    /// Who changes gear: the box (default) or the player.
    pub transmission: Transmission,
    pub camera_mode: CameraMode,
    /// How the HUD's minimap is shown: fixed (default), rotating or off.
    pub minimap: MinimapMode,
    /// Placement and scale of the in-game HUD.
    pub hud_layout: HudLayout,
    /// How the radio announces the song on the HUD: the original EA Trax chyron or the rewrite's card.
    pub radio_hud: RadioHudStyle,
    /// Post-processing, all off by default: the tone-mapping curve, bloom strength and anti-aliasing.
    pub post_tonemap: PostTonemap,
    pub post_bloom: PostBloom,
    pub post_aa: PostAa,
    pub visual_treatment: VisualTreatment,
    pub reflections: ReflectionQuality,
    /// Experimental contact occlusion; presets never enable it implicitly.
    pub ao: AoQuality,
    pub ao_intensity: Percent,
    pub ao_compare: bool,
    /// The 3D scene's size in percent of the output size per axis, and the filter that scales it back up.
    pub render_scale: RenderScale,
    pub upscaler: UpscaleMode,
    /// FSR 1 sharpening, 0 to 100 percent.
    pub upscale_sharpness: Percent,
    /// How far a temporal upscaler (fsr3, fsr4, dlss) renders below the output size.
    pub upscale_quality: UpscaleQuality,
    /// Ray-traced lighting (bevy renderer only); off to on needs a restart.
    pub ray_tracing: RayTracingLevel,
    /// Gamepad button codes of a steering wheel's shift paddles (`GamepadButton::Other`), if the player gave them.
    pub paddle_up: Option<u32>,
    /// The headlights hotkey as a Windows virtual-key code (Extra Options `[Hotkeys] Headlights`); `None` is H.
    pub headlights_key: Option<u32>,
    pub paddle_down: Option<u32>,
    /// A clutch pedal is bound (the `clutch` action): while it is pressed the clutch stays open. Off by default.
    pub manual_clutch: bool,
    /// A gear selector that holds a gear (an H-pattern shifter) is bound to the `gear_*` actions. Off by default.
    pub h_shifter: bool,
}

impl From<Partial> for Settings {
    /// Fill what no layer set with the defaults.
    fn from(p: Partial) -> Self {
        // A preset fills what no layer set: command line > environment > config file > preset > defaults.
        let requested = p.graphics_preset.unwrap_or_default();
        let p = p.or(requested.layer());
        let mut settings = Self {
            graphics_preset: requested,
            controls: Controls {
                deadzone_mode: p.deadzone_mode.unwrap_or_default(),
                steering_deadzone: p.steering_deadzone.unwrap_or(Controls::default().steering_deadzone),
                camera_deadzone: p.camera_deadzone.unwrap_or(Controls::default().camera_deadzone),
                trigger_deadzone: p.trigger_deadzone.unwrap_or(Controls::default().trigger_deadzone),
                steering_sensitivity: p.steering_sensitivity.unwrap_or(Controls::default().steering_sensitivity),
                camera_sensitivity: p.camera_sensitivity.unwrap_or(Controls::default().camera_sensitivity),
                mouse_sensitivity: p.mouse_sensitivity.unwrap_or(Controls::default().mouse_sensitivity),
                invert_camera_y: p.invert_camera_y.unwrap_or(Controls::default().invert_camera_y),
            },
            backend: p.backend.unwrap_or_default(),
            renderer: p.renderer.unwrap_or_default(),
            vsync: p.vsync.unwrap_or(true),
            max_fps: p.max_fps.unwrap_or_default(),
            show_metrics: p.show_metrics.unwrap_or_default(),
            window_mode: p.window_mode.unwrap_or_default(),
            monitor: p.monitor.unwrap_or_default(),
            resolution: p.resolution.unwrap_or_default(),
            show_readout: p.show_readout.unwrap_or_default(),
            master_volume: p.master_volume.unwrap_or(Percent(80)),
            music_volume: p.music_volume.unwrap_or(Percent(60)),
            sfx_volume: p.sfx_volume.unwrap_or(Percent(90)),
            engine_volume: p.engine_volume.unwrap_or(Percent(90)),
            speech_volume: p.speech_volume.unwrap_or(Percent(90)),
            hud: p.hud.unwrap_or(true),
            tire_smoke: p.tire_smoke.unwrap_or(true),
            radio: p.radio.unwrap_or(true),
            skip_intro: p.skip_intro.unwrap_or(false),
            car_shading: p.car_shading.unwrap_or_default(),
            traffic: p.traffic.unwrap_or(DEFAULT_TRAFFIC),
            cop_share: p.cop_share.unwrap_or(DEFAULT_COP_SHARE),
            always_rain: p.always_rain.unwrap_or(DEFAULT_ALWAYS_RAIN),
            rain_amount: p.rain_amount.unwrap_or(DEFAULT_RAIN_AMOUNT),
            game_speed: p.game_speed.unwrap_or(DEFAULT_GAME_SPEED).clamp(0.1, 4.0),
            animation_speed: p.animation_speed.unwrap_or(DEFAULT_ANIMATION_SPEED).clamp(0.1, 4.0),
            show_hud_on_look_back: p.show_hud_on_look_back.unwrap_or(DEFAULT_SHOW_HUD_ON_LOOK_BACK),
            car_scale: p.car_scale.unwrap_or(DEFAULT_CAR_SCALE).clamp(0.25, 4.0),
            no_rev_limiter: p.no_rev_limiter.unwrap_or(DEFAULT_NO_REV_LIMITER),
            traffic_lights: p.traffic_lights.unwrap_or(DEFAULT_TRAFFIC_LIGHTS),
            smoke_quality: p.smoke_quality.unwrap_or_default(),
            skid_marks: p.skid_marks.unwrap_or(true),
            collision_sparks: p.collision_sparks.unwrap_or(false),
            spark_style: p.spark_style.unwrap_or_default(),
            speed_trails: p.speed_trails.unwrap_or(false),
            exhaust_flames: p.exhaust_flames.unwrap_or(true),
            rear_view_mirror: p.rear_view_mirror.unwrap_or(true),
            transmission: p.transmission.unwrap_or_default(),
            camera_mode: p.camera_mode.unwrap_or_default(),
            minimap: p.minimap.unwrap_or_default(),
            hud_layout: p.hud_layout.unwrap_or_default(),
            radio_hud: p.radio_hud.unwrap_or_default(),
            post_tonemap: p.post_tonemap.unwrap_or_default(),
            post_bloom: p.post_bloom.unwrap_or_default(),
            post_aa: p.post_aa.unwrap_or_default(),
            visual_treatment: p.visual_treatment.unwrap_or_default(),
            reflections: p.reflections.unwrap_or(ReflectionQuality::Medium),
            ao: p.ao.unwrap_or_default(),
            ao_intensity: p.ao_intensity.unwrap_or(Percent(75)),
            ao_compare: p.ao_compare.unwrap_or(false),
            render_scale: p.render_scale.unwrap_or_default(),
            upscaler: p.upscaler.unwrap_or_default(),
            upscale_sharpness: p.upscale_sharpness.unwrap_or(Percent(80)),
            upscale_quality: p.upscale_quality.unwrap_or_default(),
            ray_tracing: p.ray_tracing.unwrap_or_default(),
            paddle_up: p.paddle_up,
            headlights_key: p.headlights_key,
            paddle_down: p.paddle_down,
            manual_clutch: p.manual_clutch.unwrap_or(false),
            h_shifter: p.h_shifter.unwrap_or(false),
        };
        settings.settle_preset();
        settings
    }
}

impl Settings {
    /// What a steering wheel's extra controls are switched on.
    pub fn wheel_options(&self) -> WheelOptions {
        WheelOptions { manual_clutch: self.manual_clutch, h_shifter: self.h_shifter }
    }

    /// The per-user config file (it may not exist yet).
    pub fn config_path() -> Option<std::path::PathBuf> {
        game_install::config_file_path(&nfsmw_data::game::SPEC)
    }

    /// Writes the settings a layer sets to the config file, keeping its other keys.
    pub fn save(changes: &Partial) -> anyhow::Result<std::path::PathBuf> {
        let path = Self::config_path().ok_or_else(|| anyhow::anyhow!("there is no per-user config folder"))?;
        write_file(&path, changes)?;
        Ok(path)
    }

    /// Resolve the settings: `cli` over the process environment over the config file over the defaults.
    pub fn load(cli: Partial) -> Self {
        let env = env::read(|name| std::env::var(name).ok());
        let config = game_install::config_file_path(&nfsmw_data::game::SPEC);
        let file = config.as_deref().map(file::read).unwrap_or_default();
        let extra = match config.as_deref().and_then(std::path::Path::parent) {
            Some(dir) => extra_options::read(&dir.join(extra_options::FILE_NAME)),
            None => Partial::default(),
        };
        cli.or(env).or(file).or(extra).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let s = Settings::from(Partial::default());
        assert_eq!((s.backend, s.vsync, s.max_fps), (GraphicsApi::Auto, true, MaxFps::default()));
        assert_eq!(s.transmission, Transmission::Automatic, "automatic, as in the original");
        assert_eq!(s.minimap, MinimapMode::Fixed, "fixed, the original's free roam mode");
        assert_eq!(s.wheel_options(), WheelOptions::default(), "no clutch pedal and no H-shifter unless asked for");
        assert_eq!((s.paddle_up, s.paddle_down), (None, None));
    }

    #[test]
    fn layers_resolve_in_order() {
        let cli = Partial { vsync: Some(false), ..Partial::default() };
        let env = env::read(|n| (n == env::BACKEND).then(|| "gl".to_owned()));
        let file = file::parse("backend = 'vulkan'\nmax_fps = 90\n", "test");
        let s: Settings = cli.or(env).or(file).into();
        assert!(!s.vsync, "command line");
        assert_eq!(s.backend, GraphicsApi::Gl, "environment over file");
        assert_eq!(s.max_fps, "90".parse().unwrap(), "file over default");
    }

    #[test]
    fn window_layers_resolve_independently_and_bad_values_fall_through() {
        let cli = Partial { window_mode: Some(WindowMode::Exclusive), ..Partial::default() };
        let env = env::read(|n| match n {
            env::WINDOW_MODE => Some("borderless".into()),
            env::MONITOR => Some("1".into()),
            env::RESOLUTION => Some("bad size".into()),
            _ => None,
        });
        let file = file::parse("window_mode = 'windowed'\nmonitor = 'primary'\nresolution = '1920x1080'", "test");
        let s: Settings = cli.or(env).or(file).into();
        assert_eq!(s.window_mode, WindowMode::Exclusive);
        assert_eq!(s.monitor, Monitor::Index(1));
        assert_eq!(s.resolution, Resolution::pixels(1920, 1080).unwrap());
        let bad = file::parse("window_mode = 'bad'\nmonitor = -1\nresolution = '0x0'", "test");
        assert_eq!(bad, Partial::default());
        assert_eq!(file::parse("monitor = 0", "test").monitor, Some(Monitor::Index(0)));
    }

    #[test]
    fn traffic_is_on_by_default_with_one_cop_in_twenty() {
        let s = Settings::from(Partial::default());
        assert_eq!((s.traffic, s.cop_share), (DEFAULT_TRAFFIC, DEFAULT_COP_SHARE));
        assert!(s.traffic > 0);
        assert_eq!(DEFAULT_COP_SHARE, Percent(5), "5 percent is one in twenty");
    }

    #[test]
    fn traffic_lights_are_on_by_default_and_come_from_every_layer() {
        assert!(Settings::from(Partial::default()).traffic_lights);
        const { assert!(DEFAULT_TRAFFIC_LIGHTS) };
        let cli = Partial { traffic_lights: Some(false), ..Partial::default() };
        let env = env::read(|n| (n == env::TRAFFIC_LIGHTS).then(|| "on".to_owned()));
        let file = file::parse("traffic_lights = false\n", "test");
        assert!(!Settings::from(cli.or(env).or(file)).traffic_lights, "the command line wins");
        assert!(Settings::from(Partial::default().or(env).or(file)).traffic_lights, "the environment over the file");
        assert!(!Settings::from(Partial::default().or(file)).traffic_lights);
        assert_eq!(file::parse("traffic_lights = 3", "test").traffic_lights, None);
    }

    #[test]
    fn the_traffic_settings_come_from_every_layer() {
        let cli = Partial { traffic: Some(0), ..Partial::default() };
        let env = env::read(|n| match n {
            env::TRAFFIC => Some("20".to_owned()),
            env::COP_SHARE => Some("10".to_owned()),
            _ => None,
        });
        let file = file::parse("traffic = 6\ncop_share = 25\n", "test");
        let s: Settings = cli.or(env).or(file).into();
        assert_eq!(
            (s.traffic, s.cop_share),
            (0, Percent(10)),
            "the command line turns traffic off, the environment sets the share"
        );
        let s: Settings = Partial::default().or(file).into();
        assert_eq!((s.traffic, s.cop_share), (6, Percent(25)));
        assert_eq!(env::read(|n| (n == env::TRAFFIC).then(|| "many".to_owned())).traffic, None);
        assert_eq!(file::parse("traffic = -3", "test").traffic, None, "a negative count is ignored");
    }
}

/// The settings a running world reads each frame: the physics clock and the animation clock rates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldTuning {
    pub game_speed: f32,
    pub animation_speed: f32,
    /// The player car's model scale, 0.25 to 4 (Extra Options `CarScale`).
    pub car_scale: f32,
    pub show_hud_on_look_back: bool,
    /// Lift the engine's rev limiter (Extra Options `NoRevLimiter`).
    pub no_rev_limiter: bool,
}

impl Default for WorldTuning {
    fn default() -> Self {
        Self {
            game_speed: DEFAULT_GAME_SPEED,
            animation_speed: DEFAULT_ANIMATION_SPEED,
            show_hud_on_look_back: DEFAULT_SHOW_HUD_ON_LOOK_BACK,
            car_scale: DEFAULT_CAR_SCALE,
            no_rev_limiter: DEFAULT_NO_REV_LIMITER,
        }
    }
}

impl From<&Settings> for WorldTuning {
    fn from(s: &Settings) -> Self {
        Self {
            game_speed: s.game_speed,
            animation_speed: s.animation_speed,
            show_hud_on_look_back: s.show_hud_on_look_back,
            car_scale: s.car_scale,
            no_rev_limiter: s.no_rev_limiter,
        }
    }
}
