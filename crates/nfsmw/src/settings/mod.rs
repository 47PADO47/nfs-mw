//! Settings, layered: command line > environment > per-user config file > defaults.
//!
//! Each source produces a [`Partial`]; [`Settings::load`] merges them. The in-game settings menu
//! (milestone 6) and the developer console write the config file layer.

#[macro_use]
mod names;

pub mod appearance;
pub use appearance::{Appearance, Quality as GraphicsQuality, VisualStyle};
pub mod availability;
pub mod bartender;
mod camera_mode;
mod car_shading;
#[cfg(test)]
mod car_shading_tests;
mod controls;
#[cfg(test)]
mod controls_tests;
pub mod custom_fonts;
pub mod custom_music;
mod defaults;
pub use defaults::*;
pub mod detail;
#[cfg(test)]
mod discord_tests;
pub mod eatrax;
mod effective;
mod env;
#[cfg(test)]
mod exhaust_flames_tests;
pub(crate) mod extra_options;
pub mod fairbusts;
pub mod fe_shadows;
#[cfg(test)]
mod fe_shadows_tests;
pub(crate) mod file;
#[cfg(test)]
mod filtering_tests;
pub use detail::{CarGeometry, DetailSettings, WorldDetail};
mod graphics;
mod latency;
pub use latency::{FrameGeneration, PresentMode, ReflexMode, Swapchain, parse_frame_latency};
mod graphics_preset;
#[cfg(test)]
mod graphics_preset_tests;
#[cfg(test)]
mod graphics_tests;
pub mod hd_content;
pub mod helicopter;
pub mod helicopter_lists;
pub mod hud;
mod hud_layout;
#[cfg(test)]
mod hud_layout_tests;
mod language;
mod minimap;
pub mod mods;
pub mod motion_blur;
#[cfg(test)]
mod motion_blur_tests;
pub mod net;
mod partial;
pub mod physics_rate;
pub use physics_rate::{DEFAULT_PHYSICS_RATE_HZ, parse_physics_rate};
pub mod pursuit_options;
pub use pursuit_options::PursuitOptions;
mod post;
#[cfg(test)]
mod post_tests;
mod radio_hud;
mod ray_tracing;
#[cfg(test)]
mod ray_tracing_tests;
#[cfg(test)]
mod rear_view_mirror_tests;
pub mod reflection_extra;
pub mod reflection_res;
#[cfg(test)]
mod reflection_res_tests;
mod renderer;
mod road_reflections;
pub use road_reflections::{RoadReflectionResolution, RoadReflections};
#[cfg(test)]
mod renderer_tests;
mod smoke_quality;
#[cfg(test)]
mod smoke_quality_tests;
mod spark_style;
mod speed_trail_style;
mod speed_units;
pub mod speedbreaker;
#[cfg(test)]
mod speedbreaker_tests;
#[cfg(test)]
pub(crate) mod test_caps;
#[cfg(test)]
mod tire_tests;
mod traffic_avoidance;
pub mod traffic_policy;
pub use traffic_policy::{PopulationPreset, TrafficPolicy, TrafficPreset};
mod traffic_settings;
mod transmission;
#[cfg(test)]
mod treatment_tests;
mod ui_style;
#[cfg(test)]
mod ui_style_tests;
mod ui_theme;
mod upscale;
mod upscale_quality;
#[cfg(test)]
mod upscale_quality_tests;
#[cfg(test)]
mod upscale_tests;
#[cfg(test)]
mod vehicle_effects_tests;
mod wheel;
pub mod xenon_effects;
pub use xenon_effects::XenonEffects;
mod window;
pub(crate) mod write;

use blackbox_gfx::GraphicsApi;

pub use blackbox_gfx::ReflectionQuality;
pub use blackbox_gfx::reflection::{MirrorTint, ReflectionSize};
pub use blackbox_gfx::{AoMethod, AoQuality, ShadowQuality, TextureFiltering};
pub use camera_mode::CameraMode;
pub use car_shading::CarShading;
pub use controls::{Controls, Deadzone, DeadzoneMode, Sensitivity};
pub use custom_music::{CustomContext, CustomMode, CustomMusic, Sources as CustomSources};
pub use graphics_preset::GraphicsPreset;
pub use hud_layout::HudLayout;
pub use language::LanguageTag;
pub use minimap::MinimapMode;
pub use motion_blur::MotionBlur;
pub use partial::{Partial, Percent, parse_bool};
pub use post::{PostAa, PostBloom, PostTonemap, VisualTreatment, post_effects};
pub use radio_hud::RadioHudStyle;
pub use ray_tracing::RayTracingLevel;
pub use renderer::RendererKind;
pub use smoke_quality::SmokeQuality;
pub use spark_style::SparkStyle;
pub use speed_trail_style::SpeedTrailStyle;
pub use speed_units::SpeedUnits;
pub use speedbreaker::SpeedbreakerTuning;
pub use traffic_settings::TrafficSettings;
pub use transmission::Transmission;
pub use ui_style::UiStyle;
pub use ui_theme::UiTheme;
pub use upscale::{RenderScale, UpscaleMode};
pub use upscale_quality::UpscaleQuality;
pub use wheel::WheelOptions;
pub use window::{Monitor, Resolution, WindowMode};
pub use write::write as write_file;

use crate::app::pacing::MaxFps;
use crate::devtools::{ShowMetrics, ShowReadout};

/// The resolved settings.
#[derive(bevy_ecs::resource::Resource, Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub traffic_policy: TrafficPolicy,
    pub pursuit_options: PursuitOptions,
    pub detail: DetailSettings,
    pub appearance: Appearance,
    pub controls: Controls,
    pub backend: GraphicsApi,
    /// Who draws: the native `blackbox` renderer (default) or `bevy`; applies after a restart.
    pub renderer: RendererKind,
    pub reflex: ReflexMode,
    pub frame_generation: FrameGeneration,
    pub vsync: bool,
    pub max_fps: MaxFps,
    pub swapchain: Swapchain,
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
    /// The Speedbreaker tunables from `NFSMWSpeedFixerSettings.ini` next to the config file. Not a [`Partial`] layer.
    pub speedbreaker: SpeedbreakerTuning,
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
    /// Active vehicle budget, including player, police and trailers (0 stops new traffic).
    pub traffic: u32,
    /// Of those cars, the share that are patrol cops, in percent.
    pub cop_share: Percent,
    /// Whether the traffic obeys traffic lights at junctions (a rewrite extension: the original has none).
    pub traffic_lights: bool,
    /// Whether the traffic goes round a car that blocks its lane (a rewrite extension: the original does not).
    pub traffic_avoidance: bool,
    /// A health bar above each cop car that chases the player (a rewrite extension; off by default).
    pub cop_health_bars: bool,
    /// Rain on every track (Extra Options `AlwaysRain`).
    pub always_rain: bool,
    /// The rain amount, 0 to 100 percent of the original.
    pub rain_amount: Percent,
    /// The physics clock multiplier, 0.1 to 4 (Extra Options `GameSpeed`).
    pub game_speed: f32,
    pub physics_rate_hz: u32,
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
    /// Experimental high-speed effect: on or off, and which effect (`speed_trail_style`).
    pub speed_trails: bool,
    pub speed_trail_style: SpeedTrailStyle,
    /// Flames at the tail pipes: gear-change blow-off and lift-off backfire. Nothing is loaded when off.
    pub exhaust_flames: bool,
    /// Show turn signals on cars while steering (a rewrite extension, off by default).
    pub turn_signals: bool,
    /// The rear-view mirror in the HUD (the rear view captured from the car, with the radar under it).
    pub rear_view_mirror: bool,
    /// Show the game in Discord Rich Presence (the local Discord client; silently skipped when it is not running).
    pub discord: bool,
    /// Show the elapsed play time in Discord Rich Presence (needs `discord`).
    pub discord_time_played: bool,
    /// Let Discord friends join a LAN game you are in from your presence (needs `discord`).
    pub discord_join: bool,
    /// Who changes gear: the box (default) or the player.
    pub transmission: Transmission,
    pub camera_mode: CameraMode,
    /// How the HUD's minimap is shown: fixed (default), rotating or off.
    pub minimap: MinimapMode,
    /// Placement and scale of the in-game HUD.
    pub hud_layout: HudLayout,
    /// The Custom HUD's options (`[custom_hud]`): preset, size, map, information, accessibility, camera, haptics, volume.
    pub hud_options: hud::HudSettings,
    /// The look of all HUD and menu UI: the original (default), Remastered (its own cluster, minimap, menus and world
    /// map), or Custom (the rewrite's own modern HUD and menus, `docs/decisions/0013-custom-hud.md`). A slot a look
    /// does not fill falls back to Original. Replaces the old `radio_hud` key (`docs/specs/ui-themes.md` section 11).
    pub ui_style: UiStyle,
    /// The unit speed is shown in on every HUD look (`speed_units`). Display only.
    pub speed_units: SpeedUnits,
    /// The language the player picked (`language`); unset, the registry's or the system's (`i18n::start`).
    pub language: Option<LanguageTag>,
    /// The palette of the token-driven HUD. Applies only when `ui_style` is not Original.
    pub ui_theme: UiTheme,
    /// Post-processing, all off by default: the tone-mapping curve, bloom strength and anti-aliasing.
    pub post_tonemap: PostTonemap,
    pub post_bloom: PostBloom,
    pub post_aa: PostAa,
    pub texture_filtering: TextureFiltering,
    pub rain_droplet_scale: f32,
    pub road_reflections: RoadReflections,
    pub road_reflection_resolution: RoadReflectionResolution,
    pub visual_treatment: VisualTreatment,
    pub reflections: ReflectionQuality,
    /// Car cube face size and rear-view mirror picture size: auto, a percentage of the base, or pixels.
    pub reflection_cube_size: ReflectionSize,
    pub reflection_mirror_size: ReflectionSize,
    /// Round the car cube faces down to a power of two.
    pub reflection_pow2: bool,
    /// Multiplies the rear-view mirror picture: off, or a colour.
    pub mirror_tint: MirrorTint,
    pub reflection_extra: reflection_extra::ReflectionExtra,
    /// Experimental contact occlusion; presets never enable it implicitly.
    pub shadows: ShadowQuality,
    pub ao: AoQuality,
    pub ao_intensity: Percent,
    pub ao_compare: bool,
    pub ao_depth_mips: bool,
    pub ao_method: AoMethod,
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
    /// The high-speed radial blur on or off (`motion_blur`); off draws none.
    pub motion_blur: bool,
    /// The radial blur's strength multiplier, 0 to 2 (`motion_blur_strength`); 1 is the curve as it is.
    pub motion_blur_strength: f32,
    /// The HD content options: HD font glyphs, the HD pointer hotspot and HD movie framing (`[hd_content]`).
    pub hd_content: hd_content::HdContent,
    /// The custom fonts: fonts from a folder, per resolution tier (`[custom_fonts]`, `docs/specs/custom-fonts.md`).
    pub custom_fonts: custom_fonts::CustomFonts,
    /// The front-end shadows share the driving shadow settings unless explicitly disabled (`docs/specs/fe-shadows.md`).
    pub fe_shadows: fe_shadows::FeShadows,
    /// The LAN play settings: name, port, snapshot rate, interpolation delay, the overlay line, the identity file.
    pub net: net::NetSettings,
    /// The restored sparks and the speed trail: thresholds, intensities, rates, caps and cutscene/nitrous/shadow options.
    pub xenon_effects: XenonEffects,
}

impl From<Partial> for Settings {
    /// Fill what no layer set with the defaults.
    fn from(p: Partial) -> Self {
        // A preset fills what no layer set: command line > environment > config file > preset > defaults.
        let requested = p.graphics_preset.unwrap_or_default();
        let p = p.or(requested.layer());
        let mut settings = Self {
            traffic_policy: p.traffic_policy.unwrap_or_default().sanitized(),
            pursuit_options: p.pursuit_options.unwrap_or_default().sanitized(),
            detail: p.detail.unwrap_or_default(),
            appearance: p.appearance.unwrap_or_default(),
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
            reflex: p.reflex.unwrap_or_default(),
            frame_generation: p.frame_generation.unwrap_or_default(),
            vsync: p.vsync.unwrap_or(true),
            max_fps: p.max_fps.unwrap_or_default(),
            swapchain: Swapchain::from_partial(&p),
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
            traffic: p.traffic.unwrap_or(DEFAULT_TRAFFIC).min(64),
            cop_share: p.cop_share.unwrap_or(DEFAULT_COP_SHARE),
            always_rain: p.always_rain.unwrap_or(DEFAULT_ALWAYS_RAIN),
            rain_amount: p.rain_amount.unwrap_or(DEFAULT_RAIN_AMOUNT),
            game_speed: p.game_speed.unwrap_or(DEFAULT_GAME_SPEED).clamp(0.1, 4.0),
            physics_rate_hz: physics_rate::resolve(p.physics_rate_hz),
            animation_speed: p.animation_speed.unwrap_or(DEFAULT_ANIMATION_SPEED).clamp(0.1, 4.0),
            show_hud_on_look_back: p.show_hud_on_look_back.unwrap_or(DEFAULT_SHOW_HUD_ON_LOOK_BACK),
            car_scale: p.car_scale.unwrap_or(DEFAULT_CAR_SCALE).clamp(0.25, 4.0),
            no_rev_limiter: p.no_rev_limiter.unwrap_or(DEFAULT_NO_REV_LIMITER),
            traffic_lights: p.traffic_lights.unwrap_or(DEFAULT_TRAFFIC_LIGHTS),
            traffic_avoidance: p.traffic_avoidance.unwrap_or(traffic_avoidance::DEFAULT_TRAFFIC_AVOIDANCE),
            cop_health_bars: p.cop_health_bars.unwrap_or(DEFAULT_COP_HEALTH_BARS),
            smoke_quality: p.smoke_quality.unwrap_or_default(),
            skid_marks: p.skid_marks.unwrap_or(true),
            collision_sparks: p.collision_sparks.unwrap_or(false),
            spark_style: p.spark_style.unwrap_or_default(),
            speed_trails: p.speed_trails.unwrap_or(false),
            speed_trail_style: p.speed_trail_style.unwrap_or_default(),
            exhaust_flames: p.exhaust_flames.unwrap_or(true),
            turn_signals: p.turn_signals.unwrap_or(false),
            rear_view_mirror: p.rear_view_mirror.unwrap_or(true),
            discord: p.discord.unwrap_or(true),
            discord_time_played: p.discord_time_played.unwrap_or(true),
            discord_join: p.discord_join.unwrap_or(true),
            transmission: p.transmission.unwrap_or_default(),
            camera_mode: p.camera_mode.unwrap_or_default(),
            minimap: p.minimap.unwrap_or_default(),
            hud_layout: p.hud_layout.unwrap_or_default(),
            hud_options: hud::HudSettings::from_layer(p.hud_options),
            ui_style: p.ui_style.unwrap_or_default(),
            speed_units: p.speed_units.unwrap_or_default(),
            language: p.language,
            ui_theme: p.ui_theme.unwrap_or_default(),
            post_tonemap: p.post_tonemap.unwrap_or_default(),
            post_bloom: p.post_bloom.unwrap_or_default(),
            post_aa: p.post_aa.unwrap_or_default(),
            texture_filtering: p.texture_filtering.unwrap_or_default(),
            rain_droplet_scale: road_reflections::finite_scale(p.rain_droplet_scale, 0.5, 2.0),
            road_reflections: p.road_reflections.unwrap_or_default(),
            road_reflection_resolution: p.road_reflection_resolution.unwrap_or_default(),
            visual_treatment: p.visual_treatment.unwrap_or_default(),
            reflections: p.reflections.unwrap_or(ReflectionQuality::Medium),
            reflection_cube_size: p.reflection_cube_size.unwrap_or_default(),
            reflection_mirror_size: p.reflection_mirror_size.unwrap_or_default(),
            reflection_pow2: p.reflection_pow2.unwrap_or(false),
            mirror_tint: p.mirror_tint.unwrap_or_default(),
            reflection_extra: reflection_extra::ReflectionExtra::from_partial(&p),
            shadows: p.shadows.unwrap_or_default(),
            ao: p.ao.unwrap_or_default(),
            ao_intensity: p.ao_intensity.unwrap_or(Percent(80)),
            ao_compare: p.ao_compare.unwrap_or(false),
            ao_depth_mips: p.ao_depth_mips.unwrap_or(false),
            ao_method: p.ao_method.unwrap_or(AoMethod::XeGtao),
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
            motion_blur: p.motion_blur.unwrap_or(motion_blur::DEFAULT_MOTION_BLUR),
            motion_blur_strength: p
                .motion_blur_strength
                .unwrap_or(motion_blur::DEFAULT_MOTION_BLUR_STRENGTH)
                .clamp(motion_blur::MOTION_BLUR_STRENGTH_RANGE.0, motion_blur::MOTION_BLUR_STRENGTH_RANGE.1),
            hd_content: hd_content::HdContent::from_layer(p.hd_content),
            custom_fonts: custom_fonts::CustomFonts::from_layer(p.custom_fonts),
            fe_shadows: fe_shadows::FeShadows::from_layer(p.fe_shadows),
            net: net::NetSettings::from_layer(p.net),
            xenon_effects: XenonEffects::from_layer(p.xenon_effects),
            speedbreaker: SpeedbreakerTuning::default(),
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
        let speedbreaker = match config.as_deref().and_then(std::path::Path::parent) {
            Some(dir) => speedbreaker::read(&dir.join(speedbreaker::FILE_NAME)),
            None => SpeedbreakerTuning::default(),
        };
        let mut settings: Self = cli.or(env).or(file).or(extra).into();
        settings.speedbreaker = speedbreaker;
        settings
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod xenon_effects_tests;

mod world_tuning;
pub use world_tuning::WorldTuning;
